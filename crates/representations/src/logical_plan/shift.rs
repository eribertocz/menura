//! The shift of a `prior_period` modifier (E4-8.4, CTX-3a.2): a query-grain period moved back
//! by `offset` units of `grain`. It is written in SQL over the query's own dates (E6-4.14), so
//! a plan carries the shift and the query dates apart: the dates are query values, bound as
//! parameters, and the amounts of the shift are model constants, written as literals
//! (E6-4.11, E6-4.12).

use std::num::NonZeroU64;

use menura_contracts::types::common::Grain;
use menura_model::expression::{
    BinaryOperator, DateUnit, DateValue, Expression, ExtractPart, IntegerLiteral, Literal,
    ScalarFunction, UnaryOperator,
};
use menura_model::internal_model::FieldPath;

use super::shift_calendar::{previous_day, shifted_start_date, start_of, start_of_next};

/// The shift of the periods of the query grain of a `prior_period` modifier (CTX-3a.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeriodShift {
    /// The query grain `g` of the time key (E4-8.6).
    pub query_grain: Grain,
    /// The modifier's `grain`, the unit of the shift.
    pub grain: Grain,
    /// The modifier's `offset`, the number of units shifted back.
    pub offset: NonZeroU64,
}

/// The shifted `date_range` of a `prior_period` context query (E4-8.4): from the first day of
/// the shifted query-grain period that contains `from` to the last day of the shifted period
/// that contains `to`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShiftedRange {
    /// The value compared: the column of the `date_range` field in the branch.
    pub column: Expression,
    /// The `date_range` field, the modifier's `time_dimension`.
    pub field: FieldPath,
    /// The query's first date, inclusive.
    pub from: DateValue,
    /// The query's last date, inclusive.
    pub to: DateValue,
    /// The shift of the query-grain periods.
    pub shift: PeriodShift,
}

impl ShiftedRange {
    /// `column >= start AND column < end`, where `start` is the first day of the shifted
    /// period that contains `from` and `end` the day after the last day of the shifted period
    /// that contains `to`, both `date` values computed from the literals `from` and `to`.
    #[must_use]
    pub fn predicate(&self) -> Expression {
        let date = |value: DateValue| Expression::Literal(Literal::Date(value));
        let start = self.shift.shifted_period_of(date(self.from));
        let end = self
            .shift
            .next_period(self.shift.shifted_period_of(date(self.to)));
        binary(
            BinaryOperator::And,
            binary(BinaryOperator::GreaterOrEqual, self.column.clone(), start),
            binary(BinaryOperator::Less, self.column.clone(), end),
        )
    }

    /// The first and last days of the shifted range, as dates; `None` when a day falls
    /// outside the years 0001–9999.
    #[must_use]
    pub fn dates(&self) -> Option<(DateValue, DateValue)> {
        self.shift.shifted_dates(self.from, self.to)
    }
}

impl PeriodShift {
    /// The start of the query-grain period whose start is `start`, shifted back (CTX-3a.2).
    /// A day shifted by months, quarters or years keeps its day number, or takes the last day
    /// of the target month when it has none, as `DATEADD` does; weeks shift by 7 days per
    /// unit. A week shifted by months or quarters is the ISO week that contains its Monday
    /// shifted as a day; a week shifted by years is the week with the same number in the
    /// shifted ISO week-numbering year, week 53 mapping to week 52 when that year has none.
    #[must_use]
    pub fn shifted(&self, start: Expression) -> Expression {
        match (self.query_grain, self.grain) {
            (Grain::Week, Grain::Month | Grain::Quarter) => {
                date_trunc(DateUnit::Week, self.back(self.grain, start))
            }
            (Grain::Week, Grain::Year) => self.week_in_shifted_year(start),
            _ => self.back(self.grain, start),
        }
    }

    /// The start of the query-grain period that contains `date`, shifted back.
    #[must_use]
    pub fn shifted_period_of(&self, date: Expression) -> Expression {
        let start = match self.query_grain {
            Grain::Day => date,
            grain => date_trunc(unit(grain), date),
        };
        self.shifted(start)
    }

    /// The last day of the shifted query-grain period that contains `date`.
    #[must_use]
    pub fn shifted_last_day_of(&self, date: Expression) -> Expression {
        let next = self.next_period(self.shifted_period_of(date));
        date_add(DateUnit::Day, negative(1), next)
    }

    /// The start of the query-grain period after the one that starts at `start`.
    #[must_use]
    pub fn next_period(&self, start: Expression) -> Expression {
        date_add(unit(self.query_grain), integer(1), start)
    }

    /// The first day of the shifted query-grain period that contains `from`, and the last day
    /// of the shifted period that contains `to`; `None` outside the years 0001–9999.
    #[must_use]
    pub fn shifted_dates(&self, from: DateValue, to: DateValue) -> Option<(DateValue, DateValue)> {
        let first = shifted_start_date(start_of(from, self.query_grain)?, *self)?;
        let last_start = shifted_start_date(start_of(to, self.query_grain)?, *self)?;
        let last = previous_day(start_of_next(last_start, self.query_grain)?)?;
        Some((first, last))
    }

    /// `value` moved back `offset` units of `grain`.
    fn back(&self, grain: Grain, value: Expression) -> Expression {
        date_add(unit(grain), negative(self.offset.get()), value)
    }

    /// The Monday of the ISO week with the number of the week that starts on `monday`, in the
    /// ISO week-numbering year `offset` years earlier, or of its last week when it has fewer.
    /// The ISO year of a week is the calendar year of its Thursday, its week 1 is the week of
    /// 4 January, and its last week is the week of 28 December.
    fn week_in_shifted_year(&self, monday: Expression) -> Expression {
        let thursday = date_add(DateUnit::Day, integer(3), monday.clone());
        let fourth_of_january = date_add(
            DateUnit::Day,
            binary(
                BinaryOperator::Subtract,
                integer(4),
                extract(ExtractPart::DayOfYear, thursday.clone()),
            ),
            thursday,
        );
        let target_fourth = self.back(Grain::Year, fourth_of_january);
        let target_last_week = extract(
            ExtractPart::Week,
            date_add(
                DateUnit::Day,
                negative(7),
                date_add(DateUnit::Year, integer(1), target_fourth.clone()),
            ),
        );
        let week = Expression::Function {
            function: ScalarFunction::Least,
            arguments: vec![extract(ExtractPart::Week, monday), target_last_week],
        };
        date_add(
            DateUnit::Day,
            binary(
                BinaryOperator::Multiply,
                integer(7),
                binary(BinaryOperator::Subtract, week, integer(1)),
            ),
            date_trunc(DateUnit::Week, target_fourth),
        )
    }
}

/// The unit of `grain`.
fn unit(grain: Grain) -> DateUnit {
    match grain {
        Grain::Day => DateUnit::Day,
        Grain::Week => DateUnit::Week,
        Grain::Month => DateUnit::Month,
        Grain::Quarter => DateUnit::Quarter,
        Grain::Year => DateUnit::Year,
    }
}

fn date_add(unit: DateUnit, amount: Expression, operand: Expression) -> Expression {
    Expression::DateAdd {
        unit,
        amount: Box::new(amount),
        operand: Box::new(operand),
    }
}

fn date_trunc(unit: DateUnit, operand: Expression) -> Expression {
    Expression::DateTrunc {
        unit,
        operand: Box::new(operand),
    }
}

fn extract(part: ExtractPart, operand: Expression) -> Expression {
    Expression::Extract {
        part,
        operand: Box::new(operand),
    }
}

fn binary(operator: BinaryOperator, left: Expression, right: Expression) -> Expression {
    Expression::Binary {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    }
}

/// The integer literal `value`; every decimal representation of a `u64` is a valid one.
fn integer(value: u64) -> Expression {
    IntegerLiteral::new(&value.to_string()).map_or(Expression::Literal(Literal::Null), |literal| {
        Expression::Literal(Literal::Integer(literal))
    })
}

/// `-value`.
fn negative(value: u64) -> Expression {
    Expression::Unary {
        operator: UnaryOperator::Minus,
        operand: Box::new(integer(value)),
    }
}

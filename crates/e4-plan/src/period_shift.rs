//! The shift of a `prior_period` modifier (E4-8.4, CTX-3a.2): a query-grain period moved back
//! by `offset` units of `grain`, written as an expression over the period's start, so that the
//! shifted time key and the shifted `date_range` bounds are computed in SQL (E6-4.14).

use std::num::NonZeroU64;

use menura_contracts::types::common::Grain;
use menura_model::expression::{
    BinaryOperator, DateUnit, Expression, ExtractPart, IntegerLiteral, Literal, ScalarFunction,
    UnaryOperator,
};

/// The shift of the periods of the query grain of a `prior_period` modifier (CTX-3a.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PeriodShift {
    /// The query grain `g` of the time key (E4-8.6).
    pub(crate) query_grain: Grain,
    /// The modifier's `grain`, the unit of the shift.
    pub(crate) grain: Grain,
    /// The modifier's `offset`, the number of units shifted back.
    pub(crate) offset: NonZeroU64,
}

impl PeriodShift {
    /// The start of the query-grain period whose start is `start`, shifted back (CTX-3a.2).
    /// A day shifted by months, quarters or years keeps its day number, or takes the last day
    /// of the target month when it has none, as `DATEADD` does; weeks shift by 7 days per
    /// unit. A week shifted by months or quarters is the ISO week that contains its Monday
    /// shifted as a day; a week shifted by years is the week with the same number in the
    /// shifted ISO week-numbering year, week 53 mapping to week 52 when that year has none.
    pub(crate) fn shifted(&self, start: Expression) -> Expression {
        match (self.query_grain, self.grain) {
            (Grain::Week, Grain::Month | Grain::Quarter) => {
                date_trunc(DateUnit::Week, self.back(self.grain, start))
            }
            (Grain::Week, Grain::Year) => self.week_in_shifted_year(start),
            _ => self.back(self.grain, start),
        }
    }

    /// The start of the query-grain period that contains `date`, shifted back.
    pub(crate) fn shifted_period_of(&self, date: Expression) -> Expression {
        let start = match self.query_grain {
            Grain::Day => date,
            grain => date_trunc(unit(grain), date),
        };
        self.shifted(start)
    }

    /// The start of the query-grain period after the one that starts at `start`.
    pub(crate) fn next_period(&self, start: Expression) -> Expression {
        date_add(unit(self.query_grain), integer(1), start)
    }

    /// `value` moved back `offset` units of `grain`.
    fn back(&self, grain: Grain, value: Expression) -> Expression {
        date_add(unit(grain), negative(self.offset), value)
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
                negative_integer(7),
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

/// The integer literal `value`.
fn integer(value: u64) -> Expression {
    let digits = value.to_string();
    // Every decimal representation of a `u64` is a valid integer literal.
    IntegerLiteral::new(&digits).map_or(Expression::Literal(Literal::Null), |literal| {
        Expression::Literal(Literal::Integer(literal))
    })
}

/// `-value`.
fn negative_integer(value: u64) -> Expression {
    Expression::Unary {
        operator: UnaryOperator::Minus,
        operand: Box::new(integer(value)),
    }
}

/// `-offset`.
fn negative(offset: NonZeroU64) -> Expression {
    negative_integer(offset.get())
}

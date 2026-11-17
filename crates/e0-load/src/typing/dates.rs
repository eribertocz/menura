//! Types the date and time functions with a part or unit (E0F-10.1).

use semantic_compiler_contracts::types::common::LogicalType;
use semantic_compiler_model::expression::{DateUnit, Expression, ExtractPart};

use super::stop::{Outcome, Stop, incompatible, untyped_null};
use super::typer::{ExpressionTyper, expect_type};
use super::value_type::{Typed, big_integer, describe, is_integer};

impl ExpressionTyper<'_> {
    /// The type of `EXTRACT`, `DATE_PART`, `DATE_TRUNC`, `DATEADD` or `DATEDIFF`.
    pub(super) fn date_function(&self, expression: &Expression) -> Outcome<Typed> {
        match expression {
            Expression::Extract { part, operand } => {
                let operand = self.type_of(operand)?;
                expect_temporal(&operand, is_time_part(*part))?;
                Ok(Typed::value(big_integer(), operand.nullable))
            }
            Expression::DateTrunc { unit, operand } => {
                let operand = self.type_of(operand)?;
                let value_type = expect_unit_operand(&operand, *unit)?;
                Ok(Typed::value(value_type, operand.nullable))
            }
            Expression::DateAdd {
                unit,
                amount,
                operand,
            } => {
                let amount = self.type_of(amount)?;
                expect_type(&amount, is_integer, "the amount of DATEADD", "an integer")?;
                let operand = self.type_of(operand)?;
                let value_type = expect_unit_operand(&operand, *unit)?;
                Ok(Typed::value(
                    value_type,
                    amount.nullable || operand.nullable,
                ))
            }
            Expression::DateDiff { unit, start, end } => {
                let start = self.type_of(start)?;
                let end = self.type_of(end)?;
                date_difference(*unit, &start, &end)
            }
            _ => Err(Stop::ReportedElsewhere),
        }
    }
}

/// `E0F-03` unless `operand` is a date or timestamp, or, for a `time_part`, a time or
/// timestamp; the literal `NULL` is accepted.
pub(super) fn expect_temporal(operand: &Typed, time_part: bool) -> Outcome<()> {
    if time_part {
        expect_type(
            operand,
            |value_type| {
                matches!(
                    value_type,
                    LogicalType::Time | LogicalType::Timestamp { .. }
                )
            },
            "the argument",
            "a time or timestamp",
        )
    } else {
        expect_type(
            operand,
            |value_type| {
                matches!(
                    value_type,
                    LogicalType::Date | LogicalType::Timestamp { .. }
                )
            },
            "the argument",
            "a date or timestamp",
        )
    }
}

fn is_time_part(part: ExtractPart) -> bool {
    matches!(
        part,
        ExtractPart::Hour | ExtractPart::Minute | ExtractPart::Second
    )
}

fn is_time_unit(unit: DateUnit) -> bool {
    matches!(unit, DateUnit::Hour | DateUnit::Minute | DateUnit::Second)
}

/// The type of the date or timestamp `operand` of `DATE_TRUNC` and `DATEADD`; `hour`,
/// `minute` and `second` need a timestamp.
fn expect_unit_operand(operand: &Typed, unit: DateUnit) -> Outcome<LogicalType> {
    expect_temporal(operand, false)?;
    let Some(value_type) = operand.logical() else {
        return untyped_null("the date or timestamp");
    };
    if is_time_unit(unit) && *value_type == LogicalType::Date {
        return incompatible("a time unit needs a timestamp, not a date".to_owned());
    }
    Ok(value_type.clone())
}

/// `DATEDIFF(unit, start, end)`: `start` and `end` of one type, a date or timestamp.
fn date_difference(unit: DateUnit, start: &Typed, end: &Typed) -> Outcome<Typed> {
    expect_temporal(start, false)?;
    expect_temporal(end, false)?;
    if let (Some(start_type), Some(end_type)) = (start.logical(), end.logical())
        && start_type != end_type
    {
        return incompatible(format!(
            "the start and end of DATEDIFF must have one type, not {} and {}",
            describe(&start.value_type),
            describe(&end.value_type)
        ));
    }
    let operand_type = start.logical().or(end.logical());
    if is_time_unit(unit) && operand_type == Some(&LogicalType::Date) {
        return incompatible("a time unit needs a timestamp, not a date".to_owned());
    }
    Ok(Typed::value(big_integer(), start.nullable || end.nullable))
}

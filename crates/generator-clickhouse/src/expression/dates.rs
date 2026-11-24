//! The date and time functions of E0F-10.1: parts, `DATEADD`, `DATEDIFF`, `TO_DATE` and
//! `TO_TIMESTAMP`. A timestamp with time zone is read on the wall clock of the query time zone,
//! as E6-7.6 truncates it, so the server time zone never matters (E6-4.10).

use menura_contracts::types::common::LogicalType;
use menura_model::expression::{DateUnit, Expression, ExtractPart, ScalarFunction};

use super::ExpressionWriter;
use crate::error::{GenerateError, InvariantViolation};

impl ExpressionWriter<'_, '_> {
    /// A part of a date or timestamp, or `TO_DATE` and `TO_TIMESTAMP`, before the cast to the
    /// result type.
    pub(super) fn date_function(
        &mut self,
        function: ScalarFunction,
        arguments: &[Expression],
    ) -> Result<String, GenerateError> {
        let [value] = self.exactly(arguments)?;
        let part = match function {
            ScalarFunction::Year => "toYear",
            ScalarFunction::Quarter => "toQuarter",
            ScalarFunction::Month => "toMonth",
            ScalarFunction::Day => "toDayOfMonth",
            ScalarFunction::DayOfYear => "toDayOfYear",
            ScalarFunction::Hour => "toHour",
            ScalarFunction::Minute => "toMinute",
            ScalarFunction::Second => "toSecond",
            ScalarFunction::ToDate => return Ok(format!("toDate32({})", self.write(value)?)),
            // `TO_TIMESTAMP` gives a timestamp without time zone, which `ClickHouse` cannot
            // express (E6-7.12).
            ScalarFunction::ToTimestamp => return Err(self.inexpressible()),
            _ => return Err(self.error(InvariantViolation::MalformedFunctionCall)),
        };
        self.date_part(part, value)
    }

    /// `EXTRACT(part FROM x)`, cast to its integer result type.
    pub(super) fn extract(
        &mut self,
        expression: &Expression,
        part: ExtractPart,
        operand: &Expression,
    ) -> Result<String, GenerateError> {
        let call = self.date_part(extract_part_function(part), operand)?;
        self.typed(expression, &call)
    }

    /// `DATEADD(unit, n, x)`: calendar arithmetic on the wall clock of the query time zone
    /// for a timestamp with time zone, converted back to UTC; on the value itself otherwise.
    pub(super) fn date_add(
        &mut self,
        expression: &Expression,
        unit: DateUnit,
        amount: &Expression,
        operand: &Expression,
    ) -> Result<String, GenerateError> {
        let zoned = self.is_zoned(operand);
        let value = self.local(operand)?;
        let amount = self.write(amount)?;
        let sum = added(unit, &amount, &value);
        let sum = if zoned {
            format!("toTimeZone({sum}, 'UTC')")
        } else {
            sum
        };
        self.typed(expression, &sum)
    }

    /// `DATEDIFF(unit, start, end)`: the number of `unit` boundaries crossed from `start` to
    /// `end`, read on the wall clock of the query time zone. ISO weeks start on Monday.
    pub(super) fn date_diff(
        &mut self,
        expression: &Expression,
        unit: DateUnit,
        start: &Expression,
        end: &Expression,
    ) -> Result<String, GenerateError> {
        let zone = self.zone_of(&[start, end]);
        let end_zone = self.zone_of(&[end]);
        let start_zone = self.zone_of(&[start]);
        let end = self.operand(end)?;
        let start = self.operand(start)?;
        let difference = match unit {
            DateUnit::Week => format!(
                "intDiv(dateDiff('day', toMonday({start}{start_zone}), \
                 toMonday({end}{end_zone})), 7)"
            ),
            _ => format!("dateDiff('{}', {start}, {end}{zone})", unit_name(unit)),
        };
        self.typed(expression, &difference)
    }

    /// `function(x, tz)` for a timestamp with time zone, read in the query time zone;
    /// `function(x)` for a date.
    fn date_part(&mut self, function: &str, operand: &Expression) -> Result<String, GenerateError> {
        let zone = self.zone_of(&[operand]);
        Ok(format!("{function}({}{zone})", self.operand(operand)?))
    }

    /// `, tz`, the time-zone argument of a call that reads `operands`, when one of them is a
    /// timestamp with time zone (E6-7.6); nothing otherwise.
    fn zone_of(&mut self, operands: &[&Expression]) -> String {
        if operands.iter().any(|operand| self.is_zoned(operand)) {
            format!(", {}", self.time_zone())
        } else {
            String::new()
        }
    }

    /// `x`, or for a timestamp with time zone `toTimeZone(x, tz)`, whose parts and calendar
    /// arithmetic are those of the query time zone.
    fn local(&mut self, operand: &Expression) -> Result<String, GenerateError> {
        let zoned = self.is_zoned(operand);
        let value = self.operand(operand)?;
        if zoned {
            Ok(format!("toTimeZone({value}, {})", self.time_zone()))
        } else {
            Ok(value)
        }
    }

    /// Whether `operand` is a timestamp with time zone.
    fn is_zoned(&self, operand: &Expression) -> bool {
        matches!(
            self.known_type(operand),
            Some(LogicalType::Timestamp {
                with_time_zone: true,
                ..
            })
        )
    }
}

/// `value` plus `amount` of `unit`. The amount of a unit of a second or more is an integer;
/// seconds may have a fraction, added as microseconds.
fn added(unit: DateUnit, amount: &str, value: &str) -> String {
    let function = match unit {
        DateUnit::Year => "addYears",
        DateUnit::Quarter => "addQuarters",
        DateUnit::Month => "addMonths",
        DateUnit::Week => "addWeeks",
        DateUnit::Day => "addDays",
        DateUnit::Hour => "addHours",
        DateUnit::Minute => "addMinutes",
        DateUnit::Second => {
            return format!("addMicroseconds({value}, toInt64(toFloat64({amount}) * 1000000))");
        }
    };
    format!("{function}({value}, toInt64({amount}))")
}

/// The `dateDiff` unit of `unit`, a literal from a closed set (E6-4.11).
fn unit_name(unit: DateUnit) -> &'static str {
    match unit {
        DateUnit::Year => "year",
        DateUnit::Quarter => "quarter",
        DateUnit::Month => "month",
        DateUnit::Week => "week",
        DateUnit::Day => "day",
        DateUnit::Hour => "hour",
        DateUnit::Minute => "minute",
        DateUnit::Second => "second",
    }
}

/// The `ClickHouse` function of an `EXTRACT` part: ISO weeks and ISO days of the week, Monday
/// being 1.
fn extract_part_function(part: ExtractPart) -> &'static str {
    match part {
        ExtractPart::Year => "toYear",
        ExtractPart::Quarter => "toQuarter",
        ExtractPart::Month => "toMonth",
        ExtractPart::Week => "toISOWeek",
        ExtractPart::Day => "toDayOfMonth",
        ExtractPart::DayOfWeek => "toDayOfWeek",
        ExtractPart::DayOfYear => "toDayOfYear",
        ExtractPart::Hour => "toHour",
        ExtractPart::Minute => "toMinute",
        ExtractPart::Second => "toSecond",
    }
}

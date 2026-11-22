//! The date and time functions of E0F-10.1: parts, `DATEADD`, `DATEDIFF`, `TO_DATE` and
//! `TO_TIMESTAMP`. A timestamp with time zone is read on the wall clock of the query time zone,
//! as E6-5.2 truncates it, so the session `TimeZone` never matters (E6-4.10).

use menura_contracts::types::common::LogicalType;
use menura_model::expression::{DateUnit, Expression, ExtractPart, ScalarFunction};

use super::ExpressionWriter;
use crate::error::{GenerateError, InvariantViolation};

impl ExpressionWriter<'_, '_> {
    /// The call of a date `function`, before the cast to its result type.
    pub(super) fn date_function(
        &mut self,
        function: ScalarFunction,
        arguments: &[Expression],
    ) -> Result<String, GenerateError> {
        let [value] = self.exactly(arguments)?;
        let part = match function {
            ScalarFunction::Year => "year",
            ScalarFunction::Quarter => "quarter",
            ScalarFunction::Month => "month",
            ScalarFunction::Day => "day",
            ScalarFunction::DayOfYear => "doy",
            ScalarFunction::Hour => "hour",
            ScalarFunction::Minute => "minute",
            ScalarFunction::Second => "second",
            ScalarFunction::ToDate => return Ok(format!("CAST({} AS DATE)", self.write(value)?)),
            ScalarFunction::ToTimestamp => {
                return Ok(format!("CAST({} AS TIMESTAMP)", self.write(value)?));
            }
            _ => return Err(self.error(InvariantViolation::MalformedFunctionCall)),
        };
        self.date_part(part, value)
    }

    /// `EXTRACT(part FROM x)` and `DATE_PART('part', x)`, a 64-bit integer.
    pub(super) fn extract(
        &mut self,
        expression: &Expression,
        part: ExtractPart,
        operand: &Expression,
    ) -> Result<String, GenerateError> {
        let call = self.date_part(extract_part_name(part), operand)?;
        self.typed(expression, &call)
    }

    /// `DATEADD(unit, n, x)`: `DuckDB` adds months to the last day of a shorter month, as
    /// E0F-10.1 requires. A timestamp with time zone is added on the wall clock and converted
    /// back.
    pub(super) fn date_add(
        &mut self,
        expression: &Expression,
        unit: DateUnit,
        amount: &Expression,
        operand: &Expression,
    ) -> Result<String, GenerateError> {
        let zoned = self.is_zoned(operand);
        let value = self.local(operand)?;
        let interval = self.interval(unit, amount)?;
        let sum = if zoned {
            let zone = self.time_zone();
            format!("({value} + {interval}) AT TIME ZONE {zone}")
        } else {
            format!("{value} + {interval}")
        };
        self.typed(expression, &sum)
    }

    /// `DATEDIFF(unit, start, end)`: the `unit` boundaries crossed from `start` to `end`, with
    /// weeks starting on Monday. `end` is written first, where it first appears.
    pub(super) fn date_diff(
        &mut self,
        expression: &Expression,
        unit: DateUnit,
        start: &Expression,
        end: &Expression,
    ) -> Result<String, GenerateError> {
        let end = self.local(end)?;
        let start = self.local(start)?;
        let difference = match unit {
            DateUnit::Year => format!("date_part('year', {end}) - date_part('year', {start})"),
            DateUnit::Quarter => month_count(&end, &start, "quarter", 4),
            DateUnit::Month => month_count(&end, &start, "month", 12),
            DateUnit::Week => format!(
                "(CAST(date_trunc('week', {end}) AS DATE) - \
                 CAST(date_trunc('week', {start}) AS DATE)) // 7"
            ),
            DateUnit::Day => format!("CAST({end} AS DATE) - CAST({start} AS DATE)"),
            DateUnit::Hour => instant_count(&end, &start, "hour", "3600000000"),
            DateUnit::Minute => instant_count(&end, &start, "minute", "60000000"),
            DateUnit::Second => instant_count(&end, &start, "second", "1000000"),
        };
        self.typed(expression, &difference)
    }

    /// `date_part('part', x)` on the wall clock of the query time zone.
    fn date_part(&mut self, part: &str, operand: &Expression) -> Result<String, GenerateError> {
        Ok(format!("date_part('{part}', {})", self.local(operand)?))
    }

    /// `x`, or `(x AT TIME ZONE $tz)` for a timestamp with time zone: its wall clock in the
    /// query time zone.
    fn local(&mut self, operand: &Expression) -> Result<String, GenerateError> {
        let zoned = self.is_zoned(operand);
        let value = self.operand(operand)?;
        if zoned {
            Ok(format!("({value} AT TIME ZONE {})", self.time_zone()))
        } else {
            Ok(value)
        }
    }

    fn is_zoned(&self, operand: &Expression) -> bool {
        matches!(
            self.known_type(operand),
            Some(LogicalType::Timestamp {
                with_time_zone: true,
                ..
            })
        )
    }

    /// The `DuckDB` interval of `amount` units, each constructor with its argument type.
    fn interval(&mut self, unit: DateUnit, amount: &Expression) -> Result<String, GenerateError> {
        let amount = self.write(amount)?;
        let integer = format!("CAST({amount} AS INTEGER)");
        Ok(match unit {
            DateUnit::Year => format!("to_years({integer})"),
            DateUnit::Quarter => format!("to_months({integer} * 3)"),
            DateUnit::Month => format!("to_months({integer})"),
            DateUnit::Week => format!("to_weeks({integer})"),
            DateUnit::Day => format!("to_days({integer})"),
            DateUnit::Hour => format!("to_hours(CAST({amount} AS BIGINT))"),
            DateUnit::Minute => format!("to_minutes(CAST({amount} AS BIGINT))"),
            DateUnit::Second => format!("to_seconds(CAST({amount} AS DOUBLE))"),
        })
    }
}

/// The difference of `year * per_year + part` between `end` and `start`.
fn month_count(end: &str, start: &str, part: &str, per_year: u8) -> String {
    let count = |value: &str| {
        format!("(date_part('year', {value}) * {per_year} + date_part('{part}', {value}))")
    };
    format!("{} - {}", count(end), count(start))
}

/// The difference of `end` and `start` truncated to `unit`, in units of `micros`
/// microseconds; exact, because both are truncated.
fn instant_count(end: &str, start: &str, unit: &str, micros: &str) -> String {
    format!(
        "(epoch_us(date_trunc('{unit}', {end})) - epoch_us(date_trunc('{unit}', {start}))) \
         // {micros}"
    )
}

/// The `DuckDB` part name of `part`: `week` is the ISO week and `isodow` the ISO day of week.
fn extract_part_name(part: ExtractPart) -> &'static str {
    match part {
        ExtractPart::Year => "year",
        ExtractPart::Quarter => "quarter",
        ExtractPart::Month => "month",
        ExtractPart::Week => "week",
        ExtractPart::Day => "day",
        ExtractPart::DayOfWeek => "isodow",
        ExtractPart::DayOfYear => "doy",
        ExtractPart::Hour => "hour",
        ExtractPart::Minute => "minute",
        ExtractPart::Second => "second",
    }
}

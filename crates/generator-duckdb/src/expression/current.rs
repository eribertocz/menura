//! `CURRENT_DATE`, `CURRENT_TIME` and `CURRENT_TIMESTAMP`, bound parameters derived from the
//! query's reference time, never read from a clock (E0F-2.3, E6-4.2).

use menura_contracts::types::common::{LogicalType, TimestampPrecision};
use menura_model::expression::Expression;
use serde_json::Value;

use super::ExpressionWriter;
use crate::error::{GenerateError, InvariantViolation};
use crate::instant::utc_text;
use crate::parameters::SharedValue;
use crate::values::date_text;

impl ExpressionWriter<'_, '_> {
    /// The reference date of the query in its time zone, one `date` parameter.
    pub(super) fn current_date(&mut self) -> Result<String, GenerateError> {
        let date = self
            .environment
            .time
            .reference_date
            .ok_or_else(|| self.error(InvariantViolation::MissingReferenceTime))?;
        self.shared_parameter(
            SharedValue::ReferenceDate,
            LogicalType::Date,
            Value::String(date_text(date)),
        )
    }

    /// `CAST(CAST($r AS TIMESTAMP WITH TIME ZONE) AT TIME ZONE CAST($tz AS VARCHAR) AS <type>)`:
    /// the wall clock of the reference instant `$r` in the query time zone, as the `timestamp`
    /// or `time` of `expression`.
    pub(super) fn current_instant(
        &mut self,
        expression: &Expression,
    ) -> Result<String, GenerateError> {
        let missing = || self.error(InvariantViolation::MissingReferenceTime);
        let reference = self
            .environment
            .time
            .reference_time
            .as_ref()
            .ok_or_else(missing)?;
        let utc = utc_text(reference).ok_or_else(missing)?;
        let instant = self.shared_parameter(
            SharedValue::ReferenceInstant,
            LogicalType::Timestamp {
                precision: TimestampPrecision::Us,
                with_time_zone: true,
            },
            Value::String(utc),
        )?;
        let zone = self.time_zone();
        self.typed(expression, &format!("{instant} AT TIME ZONE {zone}"))
    }
}

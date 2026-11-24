//! `CURRENT_DATE`, `CURRENT_TIME` and `CURRENT_TIMESTAMP`, bound parameters derived from the
//! query's reference time, never read from a clock (E0F-2.3, E6-4.2).

use menura_contracts::types::common::LogicalType;
use serde_json::Value;

use super::ExpressionWriter;
use crate::error::{GenerateError, InvariantViolation};
use crate::parameters::SharedValue;
use crate::values::date_text;

impl ExpressionWriter<'_, '_> {
    /// The reference date of the query in its time zone, one `date` parameter.
    pub(super) fn current_date(&mut self) -> Result<String, GenerateError> {
        let date = self
            .environment
            .time
            .and_then(|time| time.reference_date)
            .ok_or_else(|| self.error(InvariantViolation::MissingReferenceTime))?;
        self.shared_parameter(
            SharedValue::ReferenceDate,
            LogicalType::Date,
            Value::String(date_text(date)),
        )
    }

    /// `CURRENT_TIME` or `CURRENT_TIMESTAMP`: the wall clock of the reference instant in the
    /// query time zone, a `time` or a `timestamp` without time zone, which `ClickHouse` cannot
    /// express (E6-7.12).
    pub(super) fn current_instant(&self) -> GenerateError {
        self.inexpressible()
    }
}

//! The SQL of a bound parameter: `{pn:<type>}`, the `ClickHouse` name of its logical type at
//! every use, so that the SQL never depends on how an executor binds the value (E6-7.3,
//! E6-4.10).

use menura_contracts::types::common::LogicalType;
use serde_json::Value;

use super::ExpressionWriter;
use crate::error::{GenerateError, InvariantViolation};
use crate::parameters::SharedValue;

impl ExpressionWriter<'_, '_> {
    /// Binds `value` as a new parameter of `logical_type` and returns `{pn:<type>}`.
    pub(super) fn parameter(
        &mut self,
        logical_type: LogicalType,
        value: Value,
    ) -> Result<String, GenerateError> {
        self.parameters
            .bind(logical_type, value)
            .ok_or_else(|| self.error(InvariantViolation::UnsupportedType))
    }

    /// `{pn:<type>}` for the parameter of `shared`, bound to `logical_type` and `value` at its
    /// first appearance and reused afterwards.
    pub(super) fn shared_parameter(
        &mut self,
        shared: SharedValue,
        logical_type: LogicalType,
        value: Value,
    ) -> Result<String, GenerateError> {
        self.parameters
            .bind_shared(shared, logical_type, value)
            .ok_or_else(|| self.error(InvariantViolation::UnsupportedType))
    }
}

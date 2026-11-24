//! The SQL of a bound parameter: `{pn:<type>}`, the `ClickHouse` name of its logical type at
//! every use, so that the SQL never depends on how an executor binds the value (E6-7.3,
//! E6-4.10).

use menura_contracts::types::common::LogicalType;
use serde_json::Value;

use super::ExpressionWriter;
use crate::error::GenerateError;
use crate::parameters::SharedValue;
use crate::types::{nullable_type, physical_type};

impl ExpressionWriter<'_, '_> {
    /// Binds `value` as a new parameter of `logical_type` and returns `{pn:<type>}`.
    pub(super) fn parameter(
        &mut self,
        logical_type: LogicalType,
        value: Value,
    ) -> Result<String, GenerateError> {
        let name = self.parameter_type(&logical_type, &value)?;
        Ok(self.parameters.bind(logical_type, value, &name))
    }

    /// `{pn:<type>}` for the parameter of `shared`, bound to `logical_type` and `value` at its
    /// first appearance and reused afterwards.
    pub(super) fn shared_parameter(
        &mut self,
        shared: SharedValue,
        logical_type: LogicalType,
        value: Value,
    ) -> Result<String, GenerateError> {
        let name = self.parameter_type(&logical_type, &value)?;
        Ok(self
            .parameters
            .bind_shared(shared, logical_type, value, &name))
    }

    /// The `ClickHouse` name of a parameter of `logical_type` holding `value`: `Nullable` for
    /// a NULL value (E6-7.3, E6-7.2).
    fn parameter_type(
        &self,
        logical_type: &LogicalType,
        value: &Value,
    ) -> Result<String, GenerateError> {
        let name = if value.is_null() {
            nullable_type(logical_type)
        } else {
            physical_type(logical_type)
        };
        name.ok_or_else(|| self.type_error(logical_type))
    }
}

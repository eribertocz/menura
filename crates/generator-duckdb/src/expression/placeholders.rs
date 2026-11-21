//! The SQL of a bound parameter: its placeholder cast to the `DuckDB` name of its logical type
//! at every use, so that the SQL never depends on how an executor binds the value and `DuckDB`
//! never chooses its type among the overloads of the function or operator that reads it
//! (E6-4.10, E6-4.2).

use menura_contracts::types::common::LogicalType;
use serde_json::Value;

use super::ExpressionWriter;
use crate::error::GenerateError;
use crate::parameters::SharedValue;

impl ExpressionWriter<'_, '_> {
    /// Binds `value` as a new parameter of `logical_type` and returns
    /// `CAST($n AS <type>)`.
    pub(super) fn parameter(
        &mut self,
        logical_type: LogicalType,
        value: Value,
    ) -> Result<String, GenerateError> {
        let physical = self.physical(&logical_type)?;
        let placeholder = self.parameters.bind(logical_type, value);
        Ok(cast(&placeholder, &physical))
    }

    /// `CAST($n AS <type>)` for the parameter of `shared`, bound to `logical_type` and `value`
    /// at its first appearance and reused afterwards.
    pub(super) fn shared_parameter(
        &mut self,
        shared: SharedValue,
        logical_type: LogicalType,
        value: Value,
    ) -> Result<String, GenerateError> {
        let physical = self.physical(&logical_type)?;
        let placeholder = self.parameters.bind_shared(shared, logical_type, value);
        Ok(cast(&placeholder, &physical))
    }
}

/// `CAST(placeholder AS physical)`.
pub(super) fn cast(placeholder: &str, physical: &str) -> String {
    format!("CAST({placeholder} AS {physical})")
}

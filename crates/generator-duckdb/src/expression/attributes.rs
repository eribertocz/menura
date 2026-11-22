//! `USER_ATTRIBUTE('name')`: one bound parameter per attribute, typed as declared (E6-5.7).

use menura_contracts::types::common::LogicalType;
use menura_model::internal_model::AttributeType;
use serde_json::Value;

use super::ExpressionWriter;
use crate::error::{GenerateError, InvariantViolation};
use crate::parameters::SharedValue;
use crate::types::attribute_logical_type;
use crate::values::attribute_value;

impl ExpressionWriter<'_, '_> {
    /// The placeholder of a scalar attribute.
    pub(super) fn scalar_attribute(&mut self, name: &str) -> Result<String, GenerateError> {
        if self.is_list_attribute(name)? {
            return Err(self.error(InvariantViolation::MisplacedListAttribute));
        }
        self.attribute(name)
    }

    /// The placeholder of a list attribute, bound as a `DuckDB` list (E6-5.3).
    pub(super) fn list_attribute(&mut self, name: &str) -> Result<String, GenerateError> {
        self.attribute(name)
    }

    /// Whether the attribute `name` is declared as a list.
    pub(super) fn is_list_attribute(&self, name: &str) -> Result<bool, GenerateError> {
        Ok(matches!(self.declared_type(name)?, AttributeType::List(_)))
    }

    /// The declared type of the attribute `name`.
    pub(super) fn declared_type(&self, name: &str) -> Result<AttributeType, GenerateError> {
        self.environment
            .model
            .parts()
            .security_context
            .iter()
            .flat_map(|context| context.attributes.iter())
            .find(|attribute| attribute.name == name)
            .map(|attribute| attribute.attribute_type)
            .ok_or_else(|| self.error(InvariantViolation::UndeclaredAttribute))
    }

    fn attribute(&mut self, name: &str) -> Result<String, GenerateError> {
        let (logical_type, value) = self.bound_attribute(name)?;
        let shared = SharedValue::Attribute(name.to_owned());
        self.shared_parameter(shared, logical_type, value)
    }

    /// The logical type and CON-4 value of the attribute `name`.
    fn bound_attribute(&self, name: &str) -> Result<(LogicalType, Value), GenerateError> {
        let declared = self.declared_type(name)?;
        let value = self
            .environment
            .security
            .ok_or_else(|| self.error(InvariantViolation::MissingAttribute))?
            .attributes()
            .get(name)
            .ok_or_else(|| self.error(InvariantViolation::MissingAttribute))?;
        Ok((attribute_logical_type(declared), attribute_value(value)))
    }
}

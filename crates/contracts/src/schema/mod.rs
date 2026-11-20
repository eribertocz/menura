//! Structural validation of a contract document against its published JSON Schema (CON-1, A3).
//!
//! The schemas are the files of `spec/contracts/schemas/`, embedded at compile time, so the
//! validator and the published contract can never diverge.

mod evaluator;
mod instance;
pub(crate) mod strings;

use std::fmt;

use serde_json::Value;

use crate::contract::ContractKind;
use evaluator::Evaluator;

/// A place where a document does not satisfy its JSON Schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaViolation {
    /// JSON Pointer (RFC 6901) of the offending value inside the document. A missing required
    /// member is located at the object that lacks it.
    pub pointer: String,
    /// The keyword that the value violates.
    pub kind: SchemaViolationKind,
    /// English description of what is wrong.
    pub message: String,
}

/// The JSON Schema keyword a [`SchemaViolation`] breaks.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SchemaViolationKind {
    /// `type`: the value has another JSON type.
    Type,
    /// `const`: the value differs from the only allowed one.
    Const,
    /// `enum`: the value is not one of the allowed values.
    Enum,
    /// `minimum`.
    Minimum,
    /// `maximum`.
    Maximum,
    /// `minLength`.
    MinLength,
    /// `maxLength`.
    MaxLength,
    /// `pattern`.
    Pattern,
    /// `format`.
    Format,
    /// `minItems`: the array has too few items.
    MinItems,
    /// `maxItems`.
    MaxItems,
    /// `uniqueItems`.
    UniqueItems,
    /// `required`: the object lacks the named member.
    Required(String),
    /// `additionalProperties: false`: the member is not allowed.
    AdditionalProperties,
    /// `anyOf`: no alternative matches.
    AnyOf,
    /// `oneOf`: no alternative, or more than one, matches.
    OneOf,
    /// `not`: the forbidden alternative matches.
    Not,
    /// The `false` schema rejects every value.
    FalseSchema,
    /// The schema itself cannot be evaluated: it is not JSON, uses an unsupported keyword or has
    /// an unresolvable `$ref`. The evaluator fails closed.
    UnsupportedSchema,
}

impl fmt::Display for SchemaViolation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} at `{}`", self.message, self.pointer)
    }
}

impl std::error::Error for SchemaViolation {}

/// The published JSON Schema text of a contract.
pub(crate) fn schema_text(kind: ContractKind) -> &'static str {
    match kind {
        ContractKind::CatalogSchema => {
            include_str!("../../../../spec/contracts/schemas/catalog-schema.schema.json")
        }
        ContractKind::CatalogState => {
            include_str!("../../../../spec/contracts/schemas/catalog-state.schema.json")
        }
        ContractKind::Query => include_str!("../../../../spec/contracts/schemas/query.schema.json"),
        ContractKind::SecurityContext => {
            include_str!("../../../../spec/contracts/schemas/security-context.schema.json")
        }
        ContractKind::CompiledPlan => {
            include_str!("../../../../spec/contracts/schemas/compiled-plan.schema.json")
        }
        ContractKind::CompileError => {
            include_str!("../../../../spec/contracts/schemas/compile-error.schema.json")
        }
        ContractKind::AuditRecord => {
            include_str!("../../../../spec/contracts/schemas/audit-record.schema.json")
        }
        ContractKind::ModelDescription => {
            include_str!("../../../../spec/contracts/schemas/model-description.schema.json")
        }
        ContractKind::IntegrityChecks => {
            include_str!("../../../../spec/contracts/schemas/integrity-checks.schema.json")
        }
    }
}

/// Validates `document` against the JSON Schema of `kind` (JSON Schema 2020-12, with `format`
/// asserted).
///
/// # Errors
///
/// Returns the first [`SchemaViolation`]: keywords are evaluated in a fixed order and object
/// members in name order, so the same document always reports the same violation.
///
/// # Example
///
/// ```
/// use menura_contracts::{ContractKind, validate_structure};
///
/// let context = serde_json::json!({
///     "contract": "menura.security_context",
///     "contract_version": "1.0.0",
///     "subject": "user-1",
///     "roles": [],
///     "attributes": {},
///     "extra": true
/// });
/// let violation = validate_structure(ContractKind::SecurityContext, &context).unwrap_err();
/// assert_eq!(violation.pointer, "/extra");
/// ```
pub fn validate_structure(kind: ContractKind, document: &Value) -> Result<(), SchemaViolation> {
    let schema: Value =
        serde_json::from_str(schema_text(kind)).map_err(|error| SchemaViolation {
            pointer: String::new(),
            kind: SchemaViolationKind::UnsupportedSchema,
            message: format!(
                "the embedded {} schema is not valid JSON: {error}",
                kind.contract_name()
            ),
        })?;
    validate_against_schema(&schema, document)
}

/// Validates `instance` against a JSON Schema 2020-12 `schema` that only uses the keywords the
/// contract schemas use, with `format` asserted. Any other assertion keyword, and any `$ref`
/// outside `#/$defs/`, fails closed.
///
/// # Errors
///
/// Returns the first [`SchemaViolation`], with the same order as [`validate_structure`].
///
/// # Example
///
/// ```
/// use menura_contracts::validate_against_schema;
///
/// let schema = serde_json::json!({"type": "object", "required": ["name"]});
/// let violation = validate_against_schema(&schema, &serde_json::json!({})).unwrap_err();
/// assert_eq!(violation.pointer, "");
/// ```
pub fn validate_against_schema(schema: &Value, instance: &Value) -> Result<(), SchemaViolation> {
    Evaluator::new(schema).validate(schema, instance, "")
}

/// Every violation of `instance` against `schema`, with the keywords and the fail-closed rule of
/// [`validate_against_schema`]; empty when the instance is valid.
///
/// Violations come in keyword order, then in the member-name order of objects and the index
/// order of arrays: the first one is the one [`validate_against_schema`] returns. Callers that
/// need another order sort the list themselves.
///
/// # Example
///
/// ```
/// use menura_contracts::{SchemaViolationKind, schema_violations};
///
/// let schema = serde_json::json!({
///     "required": ["name"],
///     "properties": {"size": {"type": "integer"}},
///     "additionalProperties": false
/// });
/// let violations = schema_violations(&schema, &serde_json::json!({"size": "x", "other": 1}));
/// let kinds: Vec<_> = violations.iter().map(|violation| &violation.kind).collect();
/// assert_eq!(
///     kinds,
///     [
///         &SchemaViolationKind::Required("name".to_owned()),
///         &SchemaViolationKind::AdditionalProperties,
///         &SchemaViolationKind::Type,
///     ]
/// );
/// assert_eq!(violations[1].pointer, "/other");
/// ```
#[must_use]
pub fn schema_violations(schema: &Value, instance: &Value) -> Vec<SchemaViolation> {
    Evaluator::new(schema).violations(schema, instance, "")
}

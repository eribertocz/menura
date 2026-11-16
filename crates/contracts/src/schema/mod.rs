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

/// The first place where a document does not satisfy its JSON Schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaViolation {
    /// JSON Pointer (RFC 6901) of the offending value inside the document.
    pub pointer: String,
    /// English description of what is wrong.
    pub message: String,
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
/// use semantic_compiler_contracts::{ContractKind, validate_structure};
///
/// let context = serde_json::json!({
///     "contract": "semantic_compiler.security_context",
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
            message: format!(
                "the embedded {} schema is not valid JSON: {error}",
                kind.contract_name()
            ),
        })?;
    Evaluator::new(&schema).validate(&schema, document, "")
}

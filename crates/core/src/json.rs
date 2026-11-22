//! The JSON text of an output document, for the wrappers (ARC-5.4).

use std::fmt;

use menura_contracts::types::audit_record::AuditRecordDocument;
use menura_contracts::types::compiled_plan::{CompileErrorDocument, CompiledPlanDocument};
use menura_contracts::types::dependencies::{ModelDependenciesDocument, ModelDiffDocument};
use menura_contracts::types::descriptions::{IntegrityChecksDocument, ModelDescriptionDocument};
use menura_contracts::types::migration_report::MigrationReportDocument;
use menura_contracts::types::ordered_json::{OrderedJson, OrderedObject};

use crate::ordered_text::pretty_text;

/// An output document could not be written as JSON.
#[derive(Debug)]
pub struct OutputError(serde_json::Error);

impl fmt::Display for OutputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "the output document cannot be written as JSON: {}",
            self.0
        )
    }
}

impl std::error::Error for OutputError {}

/// The JSON text of a compile error document or validation report, indented with two spaces.
///
/// # Errors
///
/// An [`OutputError`] when the document cannot be serialized.
///
/// # Example
///
/// ```
/// use menura_core::{LoadOptions, document_json, validate};
///
/// let report = validate(b"", b"{}", &LoadOptions::default());
/// assert!(document_json(&report)?.contains("menura.validation_report"));
/// # Ok::<(), menura_core::OutputError>(())
/// ```
pub fn document_json(document: &CompileErrorDocument) -> Result<String, OutputError> {
    serde_json::to_string_pretty(document).map_err(OutputError)
}

/// The JSON text of a compiled plan, indented with two spaces. Identical plans give identical
/// text, byte for byte (PLN-2.4).
///
/// # Errors
///
/// An [`OutputError`] when the plan cannot be serialized.
pub fn plan_json(plan: &CompiledPlanDocument) -> Result<String, OutputError> {
    serde_json::to_string_pretty(plan).map_err(OutputError)
}

/// The JSON text of a model-dependency document, indented with two spaces. The same model
/// gives the same text, byte for byte (DEP-3.2).
///
/// # Errors
///
/// An [`OutputError`] when the document cannot be serialized.
pub fn dependencies_json(document: &ModelDependenciesDocument) -> Result<String, OutputError> {
    serde_json::to_string_pretty(document).map_err(OutputError)
}

/// The JSON text of a model diff, indented with two spaces.
///
/// # Errors
///
/// An [`OutputError`] when the document cannot be serialized.
pub fn diff_json(document: &ModelDiffDocument) -> Result<String, OutputError> {
    serde_json::to_string_pretty(document).map_err(OutputError)
}

/// The JSON text of a migration report, indented with two spaces; the model keeps its member
/// order and its integers their exact digits, whatever their size (MIG-3.3).
///
/// # Errors
///
/// An [`OutputError`] when the document cannot be serialized.
pub fn migration_report_json(document: &MigrationReportDocument) -> Result<String, OutputError> {
    // Every member but `model` is written by its type, then read back in order; the model is
    // left empty there and put in place as it is.
    let header = MigrationReportDocument {
        contract: document.contract,
        contract_version: document.contract_version,
        producer: document.producer.clone(),
        changes: document.changes.clone(),
        model: OrderedObject::default(),
    };
    let text = serde_json::to_string(&header).map_err(OutputError)?;
    let mut members = serde_json::from_str::<OrderedObject>(&text)
        .map_err(OutputError)?
        .members()
        .to_vec();
    for (name, value) in &mut members {
        if name == "model" {
            *value = OrderedJson::Object(document.model.clone());
        }
    }
    pretty_text(&OrderedJson::Object(OrderedObject::new(members))).map_err(OutputError)
}

/// The JSON text of a model description, indented with two spaces.
///
/// # Errors
///
/// An [`OutputError`] when the document cannot be serialized.
pub fn description_json(document: &ModelDescriptionDocument) -> Result<String, OutputError> {
    serde_json::to_string_pretty(document).map_err(OutputError)
}

/// The JSON text of an integrity-check document, indented with two spaces.
///
/// # Errors
///
/// An [`OutputError`] when the document cannot be serialized.
pub fn integrity_checks_json(document: &IntegrityChecksDocument) -> Result<String, OutputError> {
    serde_json::to_string_pretty(document).map_err(OutputError)
}

/// The JSON text of an audit record, indented with two spaces.
///
/// # Errors
///
/// An [`OutputError`] when the document cannot be serialized.
pub fn audit_record_json(document: &AuditRecordDocument) -> Result<String, OutputError> {
    serde_json::to_string_pretty(document).map_err(OutputError)
}

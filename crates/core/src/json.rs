//! The JSON text of an output document, for the wrappers (ARC-5.4).

use std::fmt;

use menura_contracts::types::compiled_plan::{CompileErrorDocument, CompiledPlanDocument};

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

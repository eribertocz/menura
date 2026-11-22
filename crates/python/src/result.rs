//! The result of every call (ARC-6.5).

use menura_core::{CompileErrorDocument, OutputError, document_json};
use pyo3::prelude::*;

use crate::errors::output_error;

/// The document of a call: the exact text the CLI writes for the same call without its final
/// line feed, whether the CLI would exit `1`, and, for `compile` with `audit: true`, the audit
/// record (ARC-6.5).
#[pyclass(name = "Result", module = "menura", frozen)]
pub(crate) struct CallResult {
    /// The output document's JSON text.
    #[pyo3(get)]
    document: String,
    /// True exactly when the CLI would exit `1` (ARC-5.4).
    #[pyo3(get)]
    is_error: bool,
    /// The audit record's JSON text when `compile` ran with `audit: true`, otherwise `None`.
    #[pyo3(get)]
    audit_record: Option<String>,
}

#[pymethods]
impl CallResult {
    fn __repr__(&self) -> String {
        format!(
            "Result(is_error={})",
            if self.is_error { "True" } else { "False" }
        )
    }
}

impl CallResult {
    /// The result of `document`, an error exactly when `is_error`.
    ///
    /// # Errors
    ///
    /// A `RuntimeError` when the document cannot be written as JSON.
    pub(crate) fn of(document: Result<String, OutputError>, is_error: bool) -> PyResult<Self> {
        Ok(Self {
            document: document.map_err(|error| output_error(&error))?,
            is_error,
            audit_record: None,
        })
    }

    /// The result of the compile error `error`.
    ///
    /// # Errors
    ///
    /// A `RuntimeError` when the document cannot be written as JSON.
    pub(crate) fn error(error: &CompileErrorDocument) -> PyResult<Self> {
        Self::of(document_json(error), true)
    }

    /// This result with the audit record `audit_record`.
    pub(crate) fn with_audit_record(self, audit_record: Option<String>) -> Self {
        Self {
            audit_record,
            ..self
        }
    }
}

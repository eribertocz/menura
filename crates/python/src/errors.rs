//! The exceptions of the package: `CompileError` (ARC-6.5), and the conversion of the core's
//! usage and output errors (ARC-6.4).

use menura_core::{OutputError, UsageError};
use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyRuntimeError, PyValueError};
use pyo3::prelude::*;

create_exception!(
    menura,
    CompileError,
    PyException,
    "Raised by `Handle.load` when the model does not load; `document` holds the compile error."
);

/// The `CompileError` whose `document` is `document`, the compile error's JSON text.
pub(crate) fn compile_error(python: Python<'_>, document: String) -> PyErr {
    let error = CompileError::new_err(document.clone());
    match error.value(python).setattr("document", document) {
        Ok(()) => error,
        Err(failure) => failure,
    }
}

/// The `ValueError` of a usage error, with the CLI's usage reason and the Python argument names
/// (ARC-6.4).
pub(crate) fn usage_error(error: &UsageError) -> PyErr {
    PyValueError::new_err(error.to_string())
}

/// The `RuntimeError` of a document that cannot be written as JSON, the CLI's exit `2`.
pub(crate) fn output_error(error: &OutputError) -> PyErr {
    PyRuntimeError::new_err(error.to_string())
}

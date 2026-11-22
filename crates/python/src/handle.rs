//! The class `Handle`: an immutable loaded model and its methods (ARC-6.5, ARC-6.5a).

use menura_core::{
    CompileResult, DescribeResult, Diagnostic, Handle, audit_record_json, compile, dependencies,
    dependencies_json, describe, description_json, explain, integrity_checks,
    integrity_checks_json, load, parse_dialect, plan_json, with_state,
};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use crate::arguments::{Document, optional_bytes};
use crate::errors::{compile_error, output_error, usage_error};
use crate::options::{compile_options, load_options};
use crate::result::CallResult;

/// A loaded model, shared between threads (ARC-6.5a).
#[pyclass(name = "Handle", module = "menura", frozen)]
pub(crate) struct PythonHandle {
    handle: Handle,
    /// The JSON text of the handle's warnings, as the `warnings` member of a compiled plan.
    #[pyo3(get)]
    warnings: String,
}

#[pymethods]
impl PythonHandle {
    /// Loads `model` against `catalog_schema`, with an optional catalog state.
    ///
    /// # Errors
    ///
    /// `CompileError` when the model does not load, `ValueError` for invalid load options.
    #[staticmethod]
    #[pyo3(signature = (model, catalog_schema, catalog_state=None, load_options=None))]
    fn load(
        python: Python<'_>,
        model: Document,
        catalog_schema: Document,
        catalog_state: Option<Document>,
        load_options: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let options = self::load_options(load_options).map_err(|error| usage_error(&error))?;
        let catalog_state = optional_bytes(catalog_state);
        let loaded = python.detach(move || {
            load(
                &model.0,
                &catalog_schema.0,
                catalog_state.as_deref(),
                &options,
            )
        });
        match loaded {
            Ok(handle) => Self::new(handle),
            Err(error) => {
                let document =
                    menura_core::document_json(&error).map_err(|failure| output_error(&failure))?;
                Err(compile_error(python, document))
            }
        }
    }

    /// A new handle with `catalog_state`; the model is not reloaded (ARC-5.1).
    fn with_state(&self, python: Python<'_>, catalog_state: Document) -> PyResult<Self> {
        Self::new(python.detach(move || with_state(&self.handle, &catalog_state.0)))
    }

    /// The compiled plan of `query`, or its compile error, and the audit record when asked.
    #[pyo3(signature = (query, security_context, compile_options=None))]
    fn compile(
        &self,
        python: Python<'_>,
        query: Document,
        security_context: Document,
        compile_options: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<CallResult> {
        let options =
            self::compile_options(compile_options).map_err(|error| usage_error(&error))?;
        let compilation =
            python.detach(move || compile(&self.handle, &query.0, &security_context.0, &options));
        let audit_record = compilation
            .audit
            .map(|record| audit_record_json(&record).map_err(|error| output_error(&error)))
            .transpose()?;
        Ok(compiled(compilation.result)?.with_audit_record(audit_record))
    }

    /// The explanation of `query`, or its compile error.
    #[pyo3(signature = (query, security_context, compile_options=None))]
    fn explain(
        &self,
        python: Python<'_>,
        query: Document,
        security_context: Document,
        compile_options: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<CallResult> {
        let options =
            self::compile_options(compile_options).map_err(|error| usage_error(&error))?;
        compiled(
            python.detach(move || explain(&self.handle, &query.0, &security_context.0, &options)),
        )
    }

    /// The model description `security_context` sees, or the compile error.
    fn describe(&self, python: Python<'_>, security_context: Document) -> PyResult<CallResult> {
        match python.detach(move || describe(&self.handle, &security_context.0)) {
            DescribeResult::Description(document) => {
                CallResult::of(description_json(&document), false)
            }
            DescribeResult::Error(error) => CallResult::error(&error),
        }
    }

    /// The integrity-check document of `dialect`, or the compile error.
    #[pyo3(signature = (dialect=Some("duckdb")))]
    fn integrity_checks(&self, python: Python<'_>, dialect: Option<&str>) -> PyResult<CallResult> {
        let dialect =
            parse_dialect(dialect.unwrap_or("duckdb")).map_err(|error| usage_error(&error))?;
        match python.detach(move || integrity_checks(&self.handle, dialect)) {
            Ok(document) => CallResult::of(integrity_checks_json(&document), false),
            Err(error) => CallResult::error(&error),
        }
    }

    /// The model-dependency document, or the compile error.
    fn dependencies(&self, python: Python<'_>) -> PyResult<CallResult> {
        match python.detach(move || dependencies(&self.handle)) {
            Ok(document) => CallResult::of(dependencies_json(&document), false),
            Err(error) => CallResult::error(&error),
        }
    }
}

impl PythonHandle {
    fn new(handle: Handle) -> PyResult<Self> {
        let warnings = warnings_json(&handle.warnings().collect::<Vec<&Diagnostic>>())?;
        Ok(Self { handle, warnings })
    }
}

/// The JSON text of `warnings`, indented with two spaces as every output document.
fn warnings_json(warnings: &[&Diagnostic]) -> PyResult<String> {
    serde_json::to_string_pretty(warnings).map_err(|error| {
        PyRuntimeError::new_err(format!(
            "the output document cannot be written as JSON: {error}"
        ))
    })
}

fn compiled(result: CompileResult) -> PyResult<CallResult> {
    match result {
        CompileResult::Plan(plan) => CallResult::of(plan_json(&plan), false),
        CompileResult::Error(error) => CallResult::error(&error),
    }
}

//! The module functions `validate`, `migrate` and `diff` (ARC-6.5).

use menura_core::{
    dependencies, diff_json, document_json, load, migrate as core_migrate, migration_report_json,
    validate as core_validate,
};
use pyo3::prelude::*;

use crate::arguments::Document;
use crate::errors::usage_error;
use crate::options::load_options;
use crate::result::CallResult;

/// The validation report of `model` against `catalog_schema`, as `menura validate`.
#[pyfunction]
#[pyo3(signature = (model, catalog_schema, load_options=None))]
pub(crate) fn validate(
    python: Python<'_>,
    model: Document,
    catalog_schema: Document,
    load_options: Option<&Bound<'_, PyAny>>,
) -> PyResult<CallResult> {
    let options = self::load_options(load_options).map_err(|error| usage_error(&error))?;
    let report = python.detach(move || core_validate(&model.0, &catalog_schema.0, &options));
    CallResult::of(document_json(&report), !report.errors.is_empty())
}

/// The migration report of `model`, or its compile error, as `menura migrate`.
#[pyfunction]
#[pyo3(signature = (model, load_options=None))]
pub(crate) fn migrate(
    python: Python<'_>,
    model: Document,
    load_options: Option<&Bound<'_, PyAny>>,
) -> PyResult<CallResult> {
    let options = self::load_options(load_options).map_err(|error| usage_error(&error))?;
    match python.detach(move || core_migrate(&model.0, &options)) {
        Ok(report) => CallResult::of(migration_report_json(&report), false),
        Err(error) => CallResult::error(&error),
    }
}

/// The diff of the dependencies of two models loaded with the same load options, old first, as
/// `menura diff`.
#[pyfunction]
#[pyo3(signature = (old_model, old_catalog_schema, new_model, new_catalog_schema, load_options=None))]
pub(crate) fn diff(
    python: Python<'_>,
    old_model: Document,
    old_catalog_schema: Document,
    new_model: Document,
    new_catalog_schema: Document,
    load_options: Option<&Bound<'_, PyAny>>,
) -> PyResult<CallResult> {
    let options = self::load_options(load_options).map_err(|error| usage_error(&error))?;
    let compared = python.detach(move || {
        let old = load(&old_model.0, &old_catalog_schema.0, None, &options)?;
        let new = load(&new_model.0, &new_catalog_schema.0, None, &options)?;
        dependencies(&old)
            .and_then(|old| dependencies(&new).and_then(|new| menura_core::diff(&old, &new)))
    });
    match compared {
        Ok(document) => CallResult::of(diff_json(&document), false),
        Err(error) => CallResult::error(&error),
    }
}

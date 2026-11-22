//! `validate`, `dependencies` and `diff` (ARC-5.5, ARC-5.6).

use std::process::ExitCode;

use super::{Loaded, load_model, load_options, read, write_document, write_error};
use crate::arguments::UsageError;
use crate::files::{DiffFiles, ModelFiles};
use menura_core::{dependencies, dependencies_json, diff, diff_json, document_json, validate};

/// Runs `validate`: writes the report and returns `0` without errors, `1` with errors.
pub(super) fn run_validate(files: &ModelFiles) -> Result<ExitCode, UsageError> {
    let model = read(&files.model)?;
    let catalog_schema = read(&files.catalog_schema)?;
    let options = load_options(files.load_options.as_deref())?;
    let report = validate(&model, &catalog_schema, &options);
    let status = if report.errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    };
    write_document(document_json(&report), status)
}

/// Runs `dependencies`: writes the model-dependency document and returns `0`, or the compile
/// error of a model that does not load and returns `1` (ARC-5.6).
pub(super) fn run_dependencies(files: &ModelFiles) -> Result<ExitCode, UsageError> {
    let options = load_options(files.load_options.as_deref())?;
    let handle = match load_model(&files.model, &files.catalog_schema, None, &options)? {
        Loaded::Handle(handle) => handle,
        Loaded::Error(error) => return write_error(&error),
    };
    match dependencies(&handle) {
        Ok(document) => write_document(dependencies_json(&document), ExitCode::SUCCESS),
        Err(error) => write_error(&error),
    }
}

/// Runs `diff`: loads both models with the same load options, old first, and writes the diff
/// of their dependencies; the compile error of the first model that does not load gives `1`
/// (ARC-5.6, DEP-4.4).
pub(super) fn run_diff(files: &DiffFiles) -> Result<ExitCode, UsageError> {
    let options = load_options(files.load_options.as_deref())?;
    let old = match load_model(&files.old_model, &files.old_catalog_schema, None, &options)? {
        Loaded::Handle(handle) => handle,
        Loaded::Error(error) => return write_error(&error),
    };
    let new = match load_model(&files.new_model, &files.new_catalog_schema, None, &options)? {
        Loaded::Handle(handle) => handle,
        Loaded::Error(error) => return write_error(&error),
    };
    let compared =
        dependencies(&old).and_then(|old| dependencies(&new).and_then(|new| diff(&old, &new)));
    match compared {
        Ok(document) => write_document(diff_json(&document), ExitCode::SUCCESS),
        Err(error) => write_error(&error),
    }
}

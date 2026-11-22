//! `describe` and `integrity-checks` (ARC-5.8).

use std::process::ExitCode;

use super::{Loaded, load_handle, load_model, load_options, read, write_document, write_error};
use crate::arguments::UsageError;
use crate::files::{DescribeFiles, IntegrityChecksFiles};
use menura_core::{
    DescribeResult, describe, description_json, integrity_checks, integrity_checks_json,
};

/// Runs `describe`: writes the model description and returns `0`, or the compile error and
/// returns `1` (ARC-5.8).
pub(super) fn run_describe(files: &DescribeFiles) -> Result<ExitCode, UsageError> {
    let security_context = read(&files.security_context)?;
    let handle = match load_handle(&files.handle)? {
        Loaded::Handle(handle) => handle,
        Loaded::Error(error) => return write_error(&error),
    };
    match describe(&handle, &security_context) {
        DescribeResult::Description(document) => {
            write_document(description_json(&document), ExitCode::SUCCESS)
        }
        DescribeResult::Error(error) => write_error(&error),
    }
}

/// Runs `integrity-checks`: writes the integrity-check document of the dialect and returns
/// `0`, or the compile error and returns `1` (ARC-5.8).
pub(super) fn run_integrity_checks(files: &IntegrityChecksFiles) -> Result<ExitCode, UsageError> {
    let model = &files.model;
    let options = load_options(model.load_options.as_deref())?;
    let handle = match load_model(&model.model, &model.catalog_schema, None, &options)? {
        Loaded::Handle(handle) => handle,
        Loaded::Error(error) => return write_error(&error),
    };
    match integrity_checks(&handle, files.dialect) {
        Ok(document) => write_document(integrity_checks_json(&document), ExitCode::SUCCESS),
        Err(error) => write_error(&error),
    }
}

//! `compile` and `explain` (ARC-5.7).

use std::fs;
use std::path::Path;
use std::process::ExitCode;

use super::{Loaded, load_handle, read, write_document, write_error};
use crate::arguments::UsageError;
use crate::files::CompileFiles;
use menura_core::{
    AuditRecordDocument, CompileOptions, CompileResult, audit_record_json, compile, explain,
    plan_json,
};

/// Runs `compile`: writes the audit record to `--audit-record` with `audit: true`, then the
/// compiled plan (exit `0`) or the compile error (exit `1`). `audit: true` without
/// `--audit-record`, or the flag without `audit: true`, is a usage error; a record that cannot
/// be written exits `2` with nothing on standard output (ARC-5.7).
pub(super) fn run_compile(files: &CompileFiles) -> Result<ExitCode, UsageError> {
    let options = compile_options(files.compile_options.as_deref())?;
    match (options.audit, &files.audit_record) {
        (true, None) => {
            return Err(UsageError(
                "the compile option `audit: true` needs `--audit-record`".to_owned(),
            ));
        }
        (false, Some(_)) => {
            return Err(UsageError(
                "`--audit-record` needs the compile option `audit: true`".to_owned(),
            ));
        }
        _ => {}
    }
    let (query, security_context) = (read(&files.query)?, read(&files.security_context)?);
    let handle = match load_handle(&files.handle)? {
        Loaded::Handle(handle) => handle,
        Loaded::Error(error) => return write_error(&error),
    };
    let compilation = compile(&handle, &query, &security_context, &options);
    if let (Some(path), Some(record)) = (&files.audit_record, &compilation.audit) {
        write_record(path, record)?;
    }
    write_result(compilation.result)
}

/// Runs `explain`: writes the explanation (exit `0`) or the compile error (exit `1`); `audit`
/// and `actor` are ignored (ARC-5.7, PLN-8.3).
pub(super) fn run_explain(files: &CompileFiles) -> Result<ExitCode, UsageError> {
    let options = compile_options(files.compile_options.as_deref())?;
    let (query, security_context) = (read(&files.query)?, read(&files.security_context)?);
    let handle = match load_handle(&files.handle)? {
        Loaded::Handle(handle) => handle,
        Loaded::Error(error) => return write_error(&error),
    };
    write_result(explain(&handle, &query, &security_context, &options))
}

/// The compile options of the file at `path`, or the defaults when no file is given
/// (ARC-5.7).
fn compile_options(path: Option<&Path>) -> Result<CompileOptions, UsageError> {
    match path {
        Some(path) => CompileOptions::from_json(&read(path)?).map_err(|error| {
            UsageError(format!("the compile options `{}`: {error}", path.display()))
        }),
        None => Ok(CompileOptions::default()),
    }
}

/// Writes the audit record `record` to the file at `path` (ARC-5.7).
fn write_record(path: &Path, record: &AuditRecordDocument) -> Result<(), UsageError> {
    let text = audit_record_json(record).map_err(|error| UsageError(error.to_string()))?;
    fs::write(path, format!("{text}\n")).map_err(|error| {
        UsageError(format!(
            "cannot write the audit record `{}`: {error}",
            path.display()
        ))
    })
}

/// Writes the plan or explanation (exit `0`) or the compile error (exit `1`).
fn write_result(result: CompileResult) -> Result<ExitCode, UsageError> {
    match result {
        CompileResult::Plan(plan) => write_document(plan_json(&plan), ExitCode::SUCCESS),
        CompileResult::Error(error) => write_error(&error),
    }
}

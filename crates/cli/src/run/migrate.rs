//! `migrate` (ARC-5.9).

use std::process::ExitCode;

use super::{load_options, read, write_document, write_error};
use crate::arguments::UsageError;
use crate::files::MigrateFiles;
use menura_core::{migrate, migration_report_json};

/// Runs `migrate`: writes the migration report and returns `0`, or the compile error of a
/// model that E0.a or E0.b rejects and returns `1`. No catalog file is read (MIG-1, ARC-5.9).
pub(super) fn run_migrate(files: &MigrateFiles) -> Result<ExitCode, UsageError> {
    let options = load_options(files.load_options.as_deref())?;
    let model = read(&files.model)?;
    match migrate(&model, &options) {
        Ok(report) => write_document(migration_report_json(&report), ExitCode::SUCCESS),
        Err(error) => write_error(&error),
    }
}

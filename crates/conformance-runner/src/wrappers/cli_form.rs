//! The CLI form: a call run as one `menura` process, its standard output and exit status
//! compared with `core` (ARC-5.4, ARC-6.9).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use menura_core::Argument;

use super::call::{CallValue, WrapperCall};
use super::difference::{Mismatch, compare_text};
use super::error::ParityError;
use super::expected::{Expected, ExpectedOutput};

/// The exit status of a command that cannot run (ARC-5.4).
const CANNOT_RUN: i32 = 2;

/// Runs calls through the `menura` binary, writing their generated files to a work directory.
#[derive(Debug, Clone)]
pub struct CliForm {
    binary: PathBuf,
    work_directory: PathBuf,
}

impl CliForm {
    /// The CLI at `binary`, writing its generated files under `work_directory`, which must
    /// exist.
    #[must_use]
    pub fn new(binary: PathBuf, work_directory: PathBuf) -> Self {
        Self {
            binary,
            work_directory,
        }
    }

    /// Runs `call` and compares what the CLI gives with `expected`.
    ///
    /// # Errors
    ///
    /// Returns [`ParityError::Io`] when a file cannot be written or the binary cannot run.
    pub fn check(
        &self,
        call: &WrapperCall,
        expected: &Expected,
    ) -> Result<Option<Mismatch>, ParityError> {
        let record = self.work_directory.join("audit-record.json");
        if record.exists() {
            fs::remove_file(&record).map_err(ParityError::io(&record))?;
        }
        let mut command = Command::new(&self.binary);
        command.arg(call.command.name().replace('_', "-"));
        for (argument, value) in &call.arguments {
            command
                .arg(flag(*argument))
                .arg(self.file(*argument, value)?);
        }
        if call.audits() {
            command.arg(flag(Argument::AuditRecord)).arg(&record);
        }
        let output = command.output().map_err(ParityError::io(&self.binary))?;
        let detail = match expected {
            Expected::Output(expected) => output_difference(expected, &output, &record)?,
            Expected::Usage(error) => usage_difference(&error.message(flag), &output),
        };
        Ok(detail.map(|detail| Mismatch {
            case: call.case.clone(),
            wrapper: "cli",
            command: call.command.name(),
            detail,
        }))
    }

    /// The file the CLI reads `value` from: the fixture file, or one written for it.
    fn file(&self, argument: Argument, value: &CallValue) -> Result<PathBuf, ParityError> {
        let contents = match value {
            CallValue::Document(document) => match &document.path {
                Some(path) => return Ok(path.clone()),
                None => document.bytes.clone(),
            },
            CallValue::Options(options) => options.to_string().into_bytes(),
        };
        let path = self
            .work_directory
            .join(format!("{}.json", argument.name()));
        fs::write(&path, contents).map_err(ParityError::io(&path))?;
        Ok(path)
    }
}

/// The CLI flag of `argument`: `--catalog-schema` for `catalog_schema` (ARC-6.7).
fn flag(argument: Argument) -> String {
    format!("--{}", argument.name().replace('_', "-"))
}

/// What differs between the CLI's output and `expected`, if anything (ARC-5.4, ARC-5.7).
fn output_difference(
    expected: &ExpectedOutput,
    output: &Output,
    record: &Path,
) -> Result<Option<String>, ParityError> {
    let status = i32::from(expected.is_error);
    if output.status.code() != Some(status) {
        return Ok(Some(format!(
            "exit status {:?}, expected {status}; standard error: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
                .lines()
                .next()
                .unwrap_or_default()
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let document = format!("{}\n", expected.document);
    if let Some(detail) = compare_text("standard output", &document, &stdout) {
        return Ok(Some(detail));
    }
    let written = if record.exists() {
        Some(fs::read_to_string(record).map_err(ParityError::io(record))?)
    } else {
        None
    };
    let expected_record = expected
        .audit_record
        .as_ref()
        .map(|text| format!("{text}\n"));
    Ok(match (expected_record, written) {
        (None, None) => None,
        (Some(expected), Some(written)) => compare_text("audit record", &expected, &written),
        (Some(_), None) => Some("no audit record was written".to_owned()),
        (None, Some(_)) => Some("an audit record was written, none expected".to_owned()),
    })
}

/// What differs between the CLI's refusal and the usage error `message` (ARC-5.4).
fn usage_difference(message: &str, output: &Output) -> Option<String> {
    if output.status.code() != Some(CANNOT_RUN) || !output.stdout.is_empty() {
        return Some(format!(
            "exit status {:?} with {} bytes of standard output, expected {CANNOT_RUN} and none",
            output.status.code(),
            output.stdout.len()
        ));
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    compare_text("standard error", &format!("menura: {message}\n"), &stderr)
}

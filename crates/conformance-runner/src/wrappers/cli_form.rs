//! The CLI form: a call run as one `menura` process, its standard output and exit status
//! compared with `core` (ARC-5.4, ARC-6.9).

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use menura_core::Argument;

use super::call::{CallValue, WrapperCall};
use super::difference::{Mismatch, compare_text};
use super::error::ParityError;
use super::expected::{Expected, ExpectedOutput};

/// The exit status of a command that cannot run (ARC-5.4).
const CANNOT_RUN: i32 = 2;

/// What one `menura` process gave: its exit status, its standard output and error as bytes, and
/// the audit record it wrote, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliOutput {
    /// The exit status, or `None` when the process was ended by a signal.
    pub status: Option<i32>,
    /// Standard output, as written.
    pub stdout: Vec<u8>,
    /// Standard error, as written.
    pub stderr: Vec<u8>,
    /// The file written to `--audit-record`, as written.
    pub audit_record: Option<Vec<u8>>,
}

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

    /// Runs `call` and compares what the CLI gives with `expected`, byte for byte.
    ///
    /// # Errors
    ///
    /// Returns [`ParityError::Io`] when a file cannot be written or the binary cannot run.
    pub fn check(
        &self,
        call: &WrapperCall,
        expected: &Expected,
    ) -> Result<Option<Mismatch>, ParityError> {
        let output = self.run(call)?;
        Ok(Self::compare(call, expected, &output))
    }

    /// The difference between what the CLI gave for `call` and `expected`, byte for byte, if
    /// any.
    #[must_use]
    pub fn compare(
        call: &WrapperCall,
        expected: &Expected,
        output: &CliOutput,
    ) -> Option<Mismatch> {
        let detail = match expected {
            Expected::Output(expected) => output_difference(expected, output),
            Expected::Usage(error) => usage_difference(&error.message(flag), output),
        }?;
        Some(Mismatch {
            case: call.case.clone(),
            wrapper: "cli",
            command: call.command.name(),
            detail,
        })
    }

    /// Runs `call` as one `menura` process and returns what it gave.
    ///
    /// # Errors
    ///
    /// Returns [`ParityError::Io`] when a file cannot be written or read, or the binary cannot
    /// run.
    pub fn run(&self, call: &WrapperCall) -> Result<CliOutput, ParityError> {
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
        let audit_record = if record.exists() {
            Some(fs::read(&record).map_err(ParityError::io(&record))?)
        } else {
            None
        };
        Ok(CliOutput {
            status: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
            audit_record,
        })
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
fn output_difference(expected: &ExpectedOutput, output: &CliOutput) -> Option<String> {
    let status = i32::from(expected.is_error);
    if output.status != Some(status) {
        return Some(format!(
            "exit status {:?}, expected {status}; standard error: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
                .lines()
                .next()
                .unwrap_or_default()
        ));
    }
    let document = format!("{}\n", expected.document);
    if let Some(detail) = byte_difference("standard output", &document, &output.stdout) {
        return Some(detail);
    }
    let expected_record = expected
        .audit_record
        .as_ref()
        .map(|text| format!("{text}\n"));
    match (expected_record, &output.audit_record) {
        (None, None) => None,
        (Some(expected), Some(written)) => byte_difference("audit record", &expected, written),
        (Some(_), None) => Some("no audit record was written".to_owned()),
        (None, Some(_)) => Some("an audit record was written, none expected".to_owned()),
    }
}

/// What differs between the bytes `actual` and the text `expected`, if anything: the first
/// differing line, or, when the difference is not visible as text, the byte counts.
fn byte_difference(label: &str, expected: &str, actual: &[u8]) -> Option<String> {
    if actual == expected.as_bytes() {
        return None;
    }
    compare_text(label, expected, &String::from_utf8_lossy(actual)).or_else(|| {
        Some(format!(
            "{label}: the bytes differ ({} expected, {} found)",
            expected.len(),
            actual.len()
        ))
    })
}

/// What differs between the CLI's refusal and the usage error `message` (ARC-5.4).
fn usage_difference(message: &str, output: &CliOutput) -> Option<String> {
    if output.status != Some(CANNOT_RUN) || !output.stdout.is_empty() {
        return Some(format!(
            "exit status {:?} with {} bytes of standard output, expected {CANNOT_RUN} and none",
            output.status,
            output.stdout.len()
        ));
    }
    byte_difference(
        "standard error",
        &format!("menura: {message}\n"),
        &output.stderr,
    )
}

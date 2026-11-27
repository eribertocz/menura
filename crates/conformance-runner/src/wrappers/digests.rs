//! The SHA-256 digests of what the CLI gives for every call (PLT-1.2). The `conformance-wrappers`
//! job writes them on Linux `x86_64`; the `platforms` job compares the digests of its own CLI with
//! them, so every platform is checked against the same bytes, not only against its own `core`.
//!
//! Standard error is not digested. A command that runs writes nothing there that is part of
//! its output (ARC-5.4), and the message of one that cannot run may quote a path, whose form is
//! native to each platform (PLT-1.5), so its bytes legitimately differ between platforms. The
//! parity check compares it with `core`'s usage message on every platform instead.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use super::call::WrapperCall;
use super::cli_form::CliOutput;
use super::difference::Mismatch;
use super::error::ParityError;

/// The digest line of one call: its case, its command, the exit status of the CLI and the
/// SHA-256 of its standard output and of its audit record (`-` when none was written).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputDigest {
    /// The case of the call.
    pub case: String,
    /// The command of the call.
    pub command: &'static str,
    /// The line written to the digest file, without its line feed.
    pub line: String,
}

impl OutputDigest {
    /// The digest of what the CLI gave for `call`.
    ///
    /// # Example
    ///
    /// ```
    /// use menura_conformance_runner::wrappers::{CliOutput, OutputDigest, WrapperCall};
    /// use menura_core::CommandName;
    ///
    /// let call = WrapperCall { case: "c-001".to_owned(), command: CommandName::Validate, arguments: vec![] };
    /// let output = CliOutput { status: Some(0), stdout: Vec::new(), stderr: Vec::new(), audit_record: None };
    /// let digest = OutputDigest::of(&call, &output);
    /// assert_eq!(
    ///     digest.line,
    ///     "c-001\tvalidate\t0\te3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\t-"
    /// );
    /// ```
    #[must_use]
    pub fn of(call: &WrapperCall, output: &CliOutput) -> Self {
        let status = output
            .status
            .map_or_else(|| "signal".to_owned(), |code| code.to_string());
        let record = output
            .audit_record
            .as_deref()
            .map_or_else(|| "-".to_owned(), sha256);
        Self {
            case: call.case.clone(),
            command: call.command.name(),
            line: format!(
                "{}\t{}\t{status}\t{}\t{record}",
                call.case,
                call.command.name(),
                sha256(&output.stdout)
            ),
        }
    }
}

/// The SHA-256 of `bytes`, as 64 lowercase hexadecimal digits.
fn sha256(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        // Writing to a `String` never fails, so the `fmt::Result` carries nothing.
        let _ = write!(text, "{byte:02x}");
    }
    text
}

/// Writes `digests` to `path`, one line each, in call order.
///
/// # Errors
///
/// Returns [`ParityError::Io`] when the file cannot be written.
pub fn write_digests(path: &Path, digests: &[OutputDigest]) -> Result<(), ParityError> {
    let mut text = String::new();
    for digest in digests {
        text.push_str(&digest.line);
        text.push('\n');
    }
    fs::write(path, text).map_err(ParityError::io(path))
}

/// The calls whose digest differs from the line of the same call in the file at `path`, which
/// [`write_digests`] wrote for the same suite; a file with another number of calls is one more
/// difference.
///
/// # Errors
///
/// Returns [`ParityError::Io`] when the file cannot be read.
pub fn digest_differences(
    path: &Path,
    digests: &[OutputDigest],
) -> Result<Vec<Mismatch>, ParityError> {
    let text = fs::read_to_string(path).map_err(ParityError::io(path))?;
    let reference: Vec<&str> = text.lines().collect();
    let mut mismatches: Vec<Mismatch> = digests
        .iter()
        .zip(&reference)
        .filter(|(digest, line)| digest.line != **line)
        .map(|(digest, line)| Mismatch {
            case: digest.case.clone(),
            wrapper: "cli digest",
            command: digest.command,
            detail: format!("expected `{line}`, found `{}`", digest.line),
        })
        .collect();
    if reference.len() != digests.len() {
        mismatches.push(Mismatch {
            case: "every case".to_owned(),
            wrapper: "cli digest",
            command: "every command",
            detail: format!(
                "{} calls in `{}`, {} here",
                reference.len(),
                path.display(),
                digests.len()
            ),
        });
    }
    Ok(mismatches)
}

#[cfg(test)]
mod tests {
    use menura_core::CommandName;

    use super::*;

    fn digest(case: &str, stdout: &[u8]) -> OutputDigest {
        let call = WrapperCall {
            case: case.to_owned(),
            command: CommandName::Compile,
            arguments: Vec::new(),
        };
        let output = CliOutput {
            status: Some(0),
            stdout: stdout.to_vec(),
            stderr: Vec::new(),
            audit_record: Some(b"{}\n".to_vec()),
        };
        OutputDigest::of(&call, &output)
    }

    fn written(test: &str, digests: &[OutputDigest]) -> std::path::PathBuf {
        // One folder per test run, so that concurrent runs never share a file.
        let folder = std::env::temp_dir()
            .join(format!("menura-digests-{}", std::process::id()))
            .join(test);
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join("digests.txt");
        write_digests(&path, digests).unwrap();
        path
    }

    #[test]
    fn plt_1_2_the_same_outputs_have_no_digest_difference() {
        let digests = [digest("a", b"{}\n"), digest("b", b"[]\n")];
        let path = written("same", &digests);
        assert!(digest_differences(&path, &digests).unwrap().is_empty());
    }

    #[test]
    fn plt_1_2_a_carriage_return_in_an_output_is_a_digest_difference() {
        let path = written("carriage_return", &[digest("a", b"{}\n")]);
        let found = digest_differences(&path, &[digest("a", b"{}\r\n")]).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].case, "a");
    }

    #[test]
    fn plt_1_2_another_number_of_calls_is_a_digest_difference() {
        let path = written("count", &[digest("a", b"{}\n")]);
        let found = digest_differences(&path, &[digest("a", b"{}\n"), digest("b", b"")]).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].case, "every case");
    }
}

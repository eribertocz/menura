//! The calls and their expected documents, written for the Python package's check
//! (`scripts/ci/check_wrapper_parity.py`, ARC-6.4, ARC-6.5, ARC-6.9).

use std::fs;
use std::path::Path;

use serde_json::{Map, Value, json};

use super::call::{CallValue, WrapperCall};
use super::error::ParityError;
use super::expected::Expected;

/// Writes `calls` and what each must give to the JSON file at `path`.
///
/// The file is `{"calls": [...]}`; each call has `case`, `command`, `arguments` and `expected`.
/// An argument is `{"path": <fixture file>}`, `{"text": <document>}` or
/// `{"options": <value>}`. `expected` is `{"document", "is_error", "audit_record"}`, or
/// `{"usage_error": <message with argument names>}`.
///
/// # Errors
///
/// Returns [`ParityError::Io`] when the file cannot be written.
pub fn write_manifest(path: &Path, calls: &[(WrapperCall, Expected)]) -> Result<(), ParityError> {
    let entries: Vec<Value> = calls
        .iter()
        .map(|(call, expected)| entry(call, expected))
        .collect();
    let text = json!({"calls": entries}).to_string();
    fs::write(path, text).map_err(ParityError::io(path))
}

/// The manifest entry of one call.
fn entry(call: &WrapperCall, expected: &Expected) -> Value {
    let mut arguments = Map::new();
    for (argument, value) in &call.arguments {
        let value = match value {
            CallValue::Document(document) => match &document.path {
                Some(path) => json!({"path": path.display().to_string()}),
                None => json!({"text": String::from_utf8_lossy(&document.bytes)}),
            },
            CallValue::Options(options) => json!({"options": options}),
        };
        arguments.insert(argument.name().to_owned(), value);
    }
    let expected = match expected {
        Expected::Output(output) => json!({
            "document": output.document,
            "is_error": output.is_error,
            "audit_record": output.audit_record,
        }),
        Expected::Usage(error) => json!({"usage_error": error.to_string()}),
    };
    json!({
        "case": call.case,
        "command": call.command.name(),
        "arguments": arguments,
        "expected": expected,
    })
}

//! The envelope checks of E1-2.1, in their order, at the locations of E1-2.2.

use menura_contracts::{IncomingContract, check_query_form};
use serde_json::Value;

use crate::error::{ParseError, ParseErrorCode};
use crate::pointer::{EXTENSIONS, TIME_ZONE};
use crate::time_zone::is_known_time_zone;

/// The only supported extension `version` (QRY-4).
const SUPPORTED_EXTENSION_VERSION: u64 = 1;

/// Runs the envelope checks on a JSON document: `EQRY-08`, `EQRY-01`, `EQRY-02`, `EQRY-06`.
pub(crate) fn check_envelope(document: &Value) -> Result<(), ParseError> {
    IncomingContract::Query.check_contract(document)?;
    check_query_form(document)?;
    check_time_zone(document)?;
    check_extension_versions(document)
}

/// `EQRY-02` for a string `time_zone` that is not a known IANA name (E1-2.3). A missing or
/// non-string `time_zone` is left to the JSON Schema.
fn check_time_zone(document: &Value) -> Result<(), ParseError> {
    match document.get("time_zone").and_then(Value::as_str) {
        Some(name) if !is_known_time_zone(name) => Err(ParseError::new(
            ParseErrorCode::Eqry02,
            TIME_ZONE,
            format!(
                "`{name}` is not the exact name of a zone or link of the IANA time-zone database"
            ),
        )),
        _ => Ok(()),
    }
}

/// `EQRY-06` for the first extension whose `data.version` is an integer other than 1. An entry
/// whose `version` cannot be read is left to the JSON Schema.
fn check_extension_versions(document: &Value) -> Result<(), ParseError> {
    let entries = document
        .get("query")
        .and_then(|query| query.get("nonportable_extensions"))
        .and_then(Value::as_array);
    for (index, entry) in entries.into_iter().flatten().enumerate() {
        let version = entry.get("data").and_then(|data| data.get("version"));
        if version.is_some_and(is_unsupported_version) {
            return Err(ParseError::new(
                ParseErrorCode::Eqry06,
                &format!("{EXTENSIONS}/{index}/data/version"),
                format!(
                    "extension version {} is not supported; the supported version is \
                     {SUPPORTED_EXTENSION_VERSION}",
                    version.map_or_else(String::new, Value::to_string)
                ),
            ));
        }
    }
    Ok(())
}

/// Whether `version` is a JSON integer, as JSON Schema reads one (`2.0` included), other than 1.
fn is_unsupported_version(version: &Value) -> bool {
    let Value::Number(number) = version else {
        return false;
    };
    if let Some(integer) = number.as_u64() {
        return integer != SUPPORTED_EXTENSION_VERSION;
    }
    if number.is_i64() {
        return true;
    }
    // An integral float other than 1 is at least 1 away from it.
    number.as_f64().is_some_and(|value| {
        value.is_finite() && value.fract() == 0.0 && (value - 1.0).abs() >= 1.0
    })
}

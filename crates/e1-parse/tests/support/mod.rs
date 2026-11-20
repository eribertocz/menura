//! Query documents and assertions shared by the E1 tests.

#![allow(
    dead_code,
    reason = "each test file uses only part of the shared support"
)]

use menura_e1_parse::{ParseError, ParseErrorCode, ParsedQuery, parse_query};
use serde_json::{Value, json};

/// A valid wide-table query with one dimension and one metric.
pub fn wide_table_query() -> Value {
    json!({
        "contract": "menura.query",
        "contract_version": "1.1.0",
        "time_zone": "Europe/Madrid",
        "query": {
            "dimensions": ["orders.region"],
            "metrics": ["revenue"]
        }
    })
}

/// `wide_table_query` with `query.nonportable_extensions` set to one entry whose `data` is
/// `data`.
pub fn with_extension(data: &Value) -> Value {
    let mut document = wide_table_query();
    document["query"]["nonportable_extensions"] =
        json!([{ "vendor_name": "COMMON", "data": *data }]);
    document
}

/// The compiler extension `data` with the members of `members` added.
pub fn extension_data(members: &Value) -> Value {
    let mut data = json!({ "namespace": "menura", "version": 1 });
    if let (Some(target), Some(source)) = (data.as_object_mut(), members.as_object()) {
        for (name, value) in source {
            target.insert(name.clone(), value.clone());
        }
    }
    data
}

/// Parses `document` and returns the result.
pub fn parse(document: &Value) -> Result<ParsedQuery, ParseError> {
    parse_query(&document.to_string())
}

/// Parses `document`, which must be accepted.
pub fn accepted(document: &Value) -> ParsedQuery {
    parse(document).expect("the query parses")
}

/// Parses `document`, which must be rejected, and returns the error.
pub fn rejected(document: &Value) -> ParseError {
    parse(document).expect_err("the query is rejected")
}

/// Asserts that `document` is rejected with `code` at `pointer`.
pub fn assert_rejected(document: &Value, code: ParseErrorCode, pointer: &str) {
    let error = rejected(document);
    assert_eq!(
        (error.code, error.pointer.as_str()),
        (code, pointer),
        "{error}"
    );
}

//! E1-2: the envelope checks, in the order of E1-2.1, at the locations of E1-2.2, and the time
//! zone names of E1-2.3.

// `cfg(test)` marks the helpers as test code, where `expect` and `panic` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_e1_parse::{ParseErrorCode, parse_query};
use serde_json::json;
use support::{
    accepted, assert_rejected, extension_data, rejected, wide_table_query, with_extension,
};

#[test]
fn e1_2_2_text_that_is_not_json_is_invalid_query_with_line_and_column() {
    let Err(error) = parse_query("{\n  \"contract\": ") else {
        panic!("expected an error");
    };
    assert_eq!(error.code, ParseErrorCode::InvalidQuery);
    assert_eq!(error.pointer, "");
    assert_eq!(error.line_column, Some((2, 14)));
}

#[test]
fn e1_2_2_missing_contract_is_eqry_08_at_contract() {
    let mut document = wide_table_query();
    document
        .as_object_mut()
        .map(|members| members.remove("contract"));
    assert_rejected(&document, ParseErrorCode::Eqry08, "/contract");
}

#[test]
fn e1_2_2_unsupported_contract_version_is_eqry_08_at_contract_version() {
    let mut document = wide_table_query();
    document["contract_version"] = json!("2.0.0");
    assert_rejected(&document, ParseErrorCode::Eqry08, "/contract_version");
}

#[test]
fn e1_2_2_both_forms_is_eqry_01_at_the_root() {
    let mut document = wide_table_query();
    document["sql"] = json!("SELECT 1");
    assert_rejected(&document, ParseErrorCode::Eqry01, "");
}

#[test]
fn e1_2_2_neither_form_is_eqry_01_at_the_root() {
    let mut document = wide_table_query();
    document
        .as_object_mut()
        .map(|members| members.remove("query"));
    assert_rejected(&document, ParseErrorCode::Eqry01, "");
}

#[test]
fn e1_2_1_contract_is_checked_before_the_form() {
    let mut document = wide_table_query();
    document["contract"] = json!("menura.catalog_schema");
    document["sql"] = json!("SELECT 1");
    assert_rejected(&document, ParseErrorCode::Eqry08, "/contract");
}

#[test]
fn e1_2_3_wrong_case_time_zone_is_eqry_02_at_time_zone() {
    let mut document = wide_table_query();
    document["time_zone"] = json!("europe/madrid");
    assert_rejected(&document, ParseErrorCode::Eqry02, "/time_zone");
}

#[test]
fn e1_2_3_unknown_time_zone_is_eqry_02() {
    let mut document = wide_table_query();
    document["time_zone"] = json!("Mars/Olympus");
    assert_rejected(&document, ParseErrorCode::Eqry02, "/time_zone");
}

#[test]
fn e1_2_3_a_link_of_the_database_is_accepted() {
    let mut document = wide_table_query();
    document["time_zone"] = json!("US/Eastern");
    let parsed = accepted(&document);
    assert_eq!(parsed.tree.time().time_zone.as_str(), "US/Eastern");
}

#[test]
fn e1_2_1_form_is_checked_before_the_time_zone() {
    let mut document = wide_table_query();
    document["time_zone"] = json!("Mars/Olympus");
    document["sql"] = json!("SELECT 1");
    assert_rejected(&document, ParseErrorCode::Eqry01, "");
}

#[test]
fn e1_2_2_missing_time_zone_is_left_to_the_json_schema() {
    let mut document = wide_table_query();
    document
        .as_object_mut()
        .map(|members| members.remove("time_zone"));
    assert_eq!(rejected(&document).code, ParseErrorCode::InvalidQuery);
}

#[test]
fn e1_2_2_non_string_time_zone_is_left_to_the_json_schema() {
    let mut document = wide_table_query();
    document["time_zone"] = json!(1);
    assert_eq!(rejected(&document).code, ParseErrorCode::InvalidQuery);
}

#[test]
fn e1_2_2_unsupported_extension_version_is_eqry_06_at_that_version() {
    let document = with_extension(&extension_data(&json!({ "version": 2 })));
    assert_rejected(
        &document,
        ParseErrorCode::Eqry06,
        "/query/nonportable_extensions/0/data/version",
    );
}

#[test]
fn e1_2_2_unreadable_extension_version_is_left_to_the_json_schema() {
    let document = with_extension(&extension_data(&json!({ "version": "1" })));
    assert_rejected(
        &document,
        ParseErrorCode::InvalidQuery,
        "/query/nonportable_extensions/0/data/version",
    );
}

#[test]
fn e1_2_1_time_zone_is_checked_before_the_extension_version() {
    let mut document = with_extension(&extension_data(&json!({ "version": 2 })));
    document["time_zone"] = json!("Mars/Olympus");
    assert_rejected(&document, ParseErrorCode::Eqry02, "/time_zone");
}

#[test]
fn e1_2_1_extension_version_is_checked_before_the_json_schema() {
    let mut document = with_extension(&extension_data(&json!({ "version": 2 })));
    document["query"]["unknown"] = json!(true);
    assert_eq!(rejected(&document).code, ParseErrorCode::Eqry06);
}

#[test]
fn e1_2_1_json_schema_failure_is_invalid_query_at_the_first_violation() {
    let mut document = wide_table_query();
    document["query"]["dimensions"] = json!([null]);
    assert_rejected(
        &document,
        ParseErrorCode::InvalidQuery,
        "/query/dimensions/0",
    );
}

#[test]
fn qry_2_reference_time_must_be_an_rfc_3339_instant() {
    let mut document = wide_table_query();
    document["reference_time"] = json!("2026-03-31 10:00:00");
    assert_rejected(&document, ParseErrorCode::InvalidQuery, "/reference_time");
}

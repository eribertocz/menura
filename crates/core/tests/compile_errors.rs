//! `compile` errors: the order across stages of E2-5 (rows 0a to 18), the compile error
//! document of PLN-5 and PLN-5.1, SEC-3.1 and SEC-3.9.

mod support;

use menura_core::{
    CompileErrorContract, CompileOptions, InputDocument, LoadOptions, compile, with_state,
};
use support::{
    CATALOG_SCHEMA, MODEL, amount_by_region, analyst, compile_default, context, error, first_code,
    handle, handle_with, plan, query,
};

/// An analyst without the `regions` attribute that `region_scope` reads (`ESEC-02`).
fn analyst_without_regions() -> String {
    context("analyst-2", &["analyst"], "{}")
}

/// A query that names a field the model does not have (`E_NAME_NOT_FOUND`).
fn unknown_name() -> String {
    query(r#"{"dimensions": ["orders.colour"], "metrics": ["total_amount"]}"#)
}

#[test]
fn e2_5_row_0a_a_security_context_that_is_not_json_is_esec_01() {
    let result = compile_default(&handle(), &amount_by_region(), "{");
    let document = error(result);
    assert_eq!(document.errors[0].code, "ESEC-01");
    let location = document.errors[0].location.as_ref().unwrap();
    assert_eq!(location.document, InputDocument::SecurityContext);
    assert_eq!(location.pointer, "");
}

#[test]
fn e2_5_row_0a_a_security_context_that_is_not_utf_8_is_esec_01_at_the_empty_pointer() {
    let result = menura_core::compile(
        &handle(),
        amount_by_region().as_bytes(),
        b"{\"subject\": \"\xff\"}",
        &CompileOptions::default(),
    )
    .result;
    let document = error(result);
    assert_eq!(document.errors[0].code, "ESEC-01");
    let location = document.errors[0].location.as_ref().unwrap();
    assert_eq!(location.document, InputDocument::SecurityContext);
    assert_eq!(location.pointer, "");
}

#[test]
fn e2_5_row_0a_an_unsupported_security_context_contract_is_esec_03() {
    let unsupported = analyst().replace("menura.security_context", "menura.query");
    assert_eq!(
        first_code(compile_default(
            &handle(),
            &amount_by_region(),
            &unsupported
        )),
        "ESEC-03"
    );
}

#[test]
fn sec_3_1_every_compilation_requires_a_security_context() {
    assert_eq!(
        first_code(compile_default(&handle(), &amount_by_region(), "")),
        "ESEC-01"
    );
}

#[test]
fn e2_5_row_0b_a_query_that_is_not_json_is_e_invalid_query() {
    let document = error(compile_default(&handle(), "{", &analyst()));
    assert_eq!(document.errors[0].code, "E_INVALID_QUERY");
    let location = document.errors[0].location.as_ref().unwrap();
    assert_eq!(location.document, InputDocument::Query);
}

#[test]
fn e2_5_row_0b_a_query_that_is_not_utf_8_is_e_invalid_query_at_the_empty_pointer() {
    let result = compile(
        &handle(),
        b"{\"time_zone\": \"\xff\"}",
        analyst().as_bytes(),
        &CompileOptions::default(),
    )
    .result;
    let document = error(result);
    assert_eq!(document.errors[0].code, "E_INVALID_QUERY");
    let location = document.errors[0].location.as_ref().unwrap();
    assert_eq!(location.document, InputDocument::Query);
    assert_eq!(location.pointer, "");
}

#[test]
fn e2_5_row_2_an_empty_query_is_an_e1_error() {
    let empty = query(r#"{"dimensions": [], "metrics": []}"#);
    assert_eq!(
        first_code(compile_default(&handle(), &empty, &analyst())),
        "E_EMPTY_AGGREGATION_QUERY"
    );
}

#[test]
fn e2_5_row_3_an_unknown_name_is_an_e2_error() {
    assert_eq!(
        first_code(compile_default(&handle(), &unknown_name(), &analyst())),
        "E_NAME_NOT_FOUND"
    );
}

#[test]
fn vs_1b_a_query_on_two_datasets_is_in_the_0_4_0_slice_and_reaches_e3() {
    let two_datasets = query(r#"{"dimensions": ["orders.region"], "metrics": ["customer_count"]}"#);
    assert_eq!(
        first_code(compile_default(&handle(), &two_datasets, &analyst())),
        "E_NO_PATH"
    );
}

#[test]
fn e2_5_row_18_a_missing_policy_attribute_is_an_e4_error() {
    let document = error(compile_default(
        &handle(),
        &amount_by_region(),
        &analyst_without_regions(),
    ));
    assert_eq!(document.errors[0].code, "ESEC-02");
}

#[test]
fn e2_5_the_security_context_comes_before_the_query() {
    assert_eq!(first_code(compile_default(&handle(), "{", "{")), "ESEC-01");
}

#[test]
fn e2_5_e1_comes_before_e2() {
    let empty_with_unknown =
        query(r#"{"dimensions": [], "metrics": [], "filter": "colour = 'x'"}"#);
    assert_eq!(
        first_code(compile_default(&handle(), &empty_with_unknown, &analyst())),
        "E_EMPTY_AGGREGATION_QUERY"
    );
}

#[test]
fn e2_5_e2_comes_before_e4() {
    assert_eq!(
        first_code(compile_default(
            &handle(),
            &unknown_name(),
            &analyst_without_regions()
        )),
        "E_NAME_NOT_FOUND"
    );
}

#[test]
fn pln_5_the_compile_error_is_a_compile_error_document_with_one_error() {
    let document = error(compile_default(&handle(), &unknown_name(), &analyst()));
    assert_eq!(document.contract, CompileErrorContract::CompileError);
    assert_eq!(document.errors.len(), 1);
    let json: serde_json::Value =
        serde_json::from_str(&menura_core::document_json(&document).unwrap()).unwrap();
    menura_contracts::validate_structure(menura_contracts::ContractKind::CompileError, &json)
        .unwrap();
}

/// The `shop` documents with former contract names: the catalog schema (`WCAT-05` at load),
/// the catalog state (`WCAT-05` after the load warnings), the security context and the query
/// (`WQRY-01` each).
fn former_name_handle() -> menura_core::Handle {
    let catalog_schema =
        CATALOG_SCHEMA.replace("menura.catalog_schema", "semantic_compiler.catalog_schema");
    let loaded = handle_with(MODEL, &catalog_schema, None, &LoadOptions::default());
    let state = r#"{"contract": "semantic_compiler.catalog_state", "contract_version": "1.1.0",
        "document_id": {"origin": "warehouse:shop", "snapshot": "state-1"},
        "schema_document_id": {"origin": "warehouse:shop", "snapshot": "schema-1"},
        "relations": []}"#;
    with_state(&loaded, state.as_bytes())
}

fn warning_locations(warnings: &[menura_core::Diagnostic]) -> Vec<(String, Option<InputDocument>)> {
    warnings
        .iter()
        .map(|warning| {
            let document = warning.location.as_ref().map(|location| location.document);
            (warning.code.clone(), document)
        })
        .collect()
}

fn former_name_warnings() -> Vec<(String, Option<InputDocument>)> {
    [
        ("WCAT-05", InputDocument::CatalogSchema),
        ("WCAT-05", InputDocument::CatalogState),
        ("WQRY-01", InputDocument::SecurityContext),
        ("WQRY-01", InputDocument::Query),
    ]
    .into_iter()
    .map(|(code, document)| (code.to_owned(), Some(document)))
    .collect()
}

#[test]
fn pln_5_the_warnings_of_an_error_are_the_load_then_the_context_then_the_query() {
    let former_context = analyst_without_regions().replace(
        "menura.security_context",
        "semantic_compiler.security_context",
    );
    let former_query = amount_by_region().replace("menura.query", "semantic_compiler.query");
    let document = error(compile_default(
        &former_name_handle(),
        &former_query,
        &former_context,
    ));
    assert_eq!(document.errors[0].code, "ESEC-02");
    assert_eq!(
        warning_locations(&document.warnings),
        former_name_warnings()
    );
}

#[test]
fn sec_3_9_the_warnings_of_a_plan_are_the_load_then_the_context_then_the_query() {
    let former_context = analyst().replace(
        "menura.security_context",
        "semantic_compiler.security_context",
    );
    let former_query = amount_by_region().replace("menura.query", "semantic_compiler.query");
    let compiled = plan(compile_default(
        &former_name_handle(),
        &former_query,
        &former_context,
    ));
    assert_eq!(
        warning_locations(&compiled.warnings),
        former_name_warnings()
    );
}

#[test]
fn pln_5_1_an_error_of_the_security_context_keeps_only_the_load_warnings() {
    let document = error(compile_default(
        &former_name_handle(),
        &amount_by_region(),
        "{",
    ));
    assert_eq!(
        warning_locations(&document.warnings),
        former_name_warnings()[..2].to_vec()
    );
}

/// The warnings of the compile error of the former-name `query` text, and its first code.
fn former_query_error(query_text: &str) -> (String, Vec<(String, Option<InputDocument>)>) {
    let former_query = query_text.replace("\"menura.query\"", "\"semantic_compiler.query\"");
    let document = error(compile_default(&handle(), &former_query, &analyst()));
    (
        document.errors[0].code.clone(),
        warning_locations(&document.warnings),
    )
}

fn former_query_warning() -> Vec<(String, Option<InputDocument>)> {
    vec![("WQRY-01".to_owned(), Some(InputDocument::Query))]
}

#[test]
fn pln_5_an_empty_query_keeps_the_former_name_warning() {
    let empty = query(r#"{"dimensions": [], "metrics": []}"#);
    assert_eq!(
        former_query_error(&empty),
        (
            "E_EMPTY_AGGREGATION_QUERY".to_owned(),
            former_query_warning()
        )
    );
}

#[test]
fn pln_5_a_filter_text_error_keeps_the_former_name_warning() {
    let bad_filter = query(r#"{"dimensions": ["orders.region"], "filter": "region = "}"#);
    assert_eq!(
        former_query_error(&bad_filter),
        ("E0F-01".to_owned(), former_query_warning())
    );
}

#[test]
fn pln_5_the_sql_form_keeps_the_former_name_warning() {
    let sql_form = r#"{"contract": "menura.query", "contract_version": "1.1.0", "time_zone": "UTC",
        "sql": "SELECT region FROM orders"}"#;
    assert_eq!(
        former_query_error(sql_form),
        ("EQRY-05".to_owned(), former_query_warning())
    );
}

#[test]
fn pln_5_a_query_that_fails_the_json_schema_has_no_query_warning() {
    let invalid = query(r#"{"dimensions": ["orders.region"], "unknown": true}"#);
    let (code, warnings) = former_query_error(&invalid);
    assert_eq!(code, "E_INVALID_QUERY");
    assert!(warnings.is_empty());
}

#[test]
fn pln_5_an_error_document_with_warnings_validates_against_its_schema() {
    let former_context = analyst_without_regions().replace(
        "menura.security_context",
        "semantic_compiler.security_context",
    );
    let document = error(compile_default(
        &former_name_handle(),
        &amount_by_region(),
        &former_context,
    ));
    assert!(!document.warnings.is_empty());
    let json: serde_json::Value =
        serde_json::from_str(&menura_core::document_json(&document).unwrap()).unwrap();
    menura_contracts::validate_structure(menura_contracts::ContractKind::CompileError, &json)
        .unwrap();
}

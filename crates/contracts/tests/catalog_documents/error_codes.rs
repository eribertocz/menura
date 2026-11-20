//! CAT-9: error codes and their order.

use menura_contracts::{AdmissionCode, admit_catalog_schema, admit_catalog_state};
use serde_json::json;

use super::{
    admitted_schema, pointer, schema_example, schema_failure, state_example, state_failure, with,
};

#[test]
fn cat_9_1_a_schema_that_is_not_json_is_ecat_01() {
    let error = admit_catalog_schema("{\"contract\": ").expect_err("not JSON");
    assert_eq!(error.code, AdmissionCode::Ecat01);
    assert!(error.line_column.is_some());
}

#[test]
fn cat_9_1_a_state_that_is_not_json_is_wcat_01() {
    let error = admit_catalog_state("[", &admitted_schema()).expect_err("not JSON");
    assert_eq!(error.code, AdmissionCode::Wcat01);
}

#[test]
fn cat_9_1_the_contract_version_is_checked_before_the_json_schema() {
    let schema = with(schema_example(), "/contract_version", json!("0.9.0"));
    let schema = with(schema, "/relations", json!({}));
    assert_eq!(
        schema_failure(&schema),
        (AdmissionCode::Ecat02, pointer("/contract_version"))
    );
    let state = with(state_example(), "/contract", json!("menura.query"));
    let state = with(state, "/relations", json!({}));
    assert_eq!(
        state_failure(&state),
        (AdmissionCode::Wcat02, pointer("/contract"))
    );
}

#[test]
fn cat_9_1_the_schema_document_id_is_checked_before_the_semantic_rules() {
    // A repeated source (CAT-8.1) in a state for another schema: `WCAT-04` is reported.
    let repeated = with(
        state_example(),
        "/relations/1/source",
        json!("sales.public.fact_invoice"),
    );
    let other_schema = with(
        repeated.clone(),
        "/schema_document_id/snapshot",
        json!("schema-0043"),
    );
    assert_eq!(
        state_failure(&other_schema),
        (AdmissionCode::Wcat04, pointer("/schema_document_id"))
    );
    // The same violation in a state for the schema in use is `WCAT-03`.
    assert_eq!(
        state_failure(&repeated),
        (AdmissionCode::Wcat03, pointer("/relations/1/source"))
    );
}

#[test]
fn cat_9_state_codes_are_warnings_and_schema_codes_are_errors() {
    let warnings = [
        AdmissionCode::Wcat01,
        AdmissionCode::Wcat02,
        AdmissionCode::Wcat03,
        AdmissionCode::Wcat04,
    ];
    let errors = [
        AdmissionCode::Ecat01,
        AdmissionCode::Ecat02,
        AdmissionCode::Ecat03,
    ];
    assert!(warnings.iter().all(|code| code.is_warning()));
    assert!(errors.iter().all(|code| !code.is_warning()));
    let texts: Vec<&str> = warnings
        .iter()
        .chain(errors.iter())
        .map(|code| code.as_str())
        .collect();
    assert_eq!(
        texts,
        [
            "WCAT-01", "WCAT-02", "WCAT-03", "WCAT-04", "ECAT-01", "ECAT-02", "ECAT-03"
        ]
    );
}

//! The former contract name: CON-0.5 and WCAT-05.

use menura_contracts::types::compiled_plan::InputDocument;
use menura_contracts::{
    AdmissionCode, admit_catalog_state, schema_former_name_warning, state_former_name_warning,
};
use serde_json::json;

use super::{
    admit_schema, admitted_schema, pointer, schema_example, schema_failure, state_example, with,
};

#[test]
fn con_0_5_wcat_05_a_catalog_schema_with_the_former_name_is_admitted_with_a_warning() {
    let former = with(
        schema_example(),
        "/contract",
        json!("semantic_compiler.catalog_schema"),
    );
    let schema = admit_schema(&former).expect("the former name is admitted");
    let warning = schema_former_name_warning(&schema).expect("WCAT-05");
    assert_eq!(warning.code, AdmissionCode::Wcat05);
    assert!(warning.code.is_warning());
    assert_eq!(warning.document, InputDocument::CatalogSchema);
    assert_eq!(warning.pointer, "/contract");
    assert_eq!(warning.line_column, None);
    let menura = with(
        schema_example(),
        "/contract",
        json!("menura.catalog_schema"),
    );
    let menura = with(menura, "/contract_version", json!("1.1.0"));
    let schema = admit_schema(&menura).expect("the name is admitted");
    assert_eq!(schema_former_name_warning(&schema), None);
}

#[test]
fn con_0_5_wcat_05_a_catalog_state_with_the_former_name_is_admitted_with_a_warning() {
    let schema = admitted_schema();
    let former = with(
        state_example(),
        "/contract",
        json!("semantic_compiler.catalog_state"),
    );
    let state = admit_catalog_state(&former.to_string(), &schema).expect("admitted");
    let warning = state_former_name_warning(&state).expect("WCAT-05");
    assert_eq!(warning.code, AdmissionCode::Wcat05);
    assert_eq!(warning.document, InputDocument::CatalogState);
    assert_eq!(warning.pointer, "/contract");
    let menura = with(state_example(), "/contract", json!("menura.catalog_state"));
    let state = admit_catalog_state(&menura.to_string(), &schema).expect("admitted");
    assert_eq!(state_former_name_warning(&state), None);
}

#[test]
fn con_0_5_a_former_name_does_not_hide_a_later_failure() {
    let former = with(
        schema_example(),
        "/contract",
        json!("semantic_compiler.catalog_schema"),
    );
    let broken = with(former, "/engine/name", json!(42));
    assert_eq!(
        schema_failure(&broken),
        (AdmissionCode::Ecat01, pointer("/engine/name"))
    );
}

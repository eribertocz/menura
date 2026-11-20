//! The fields and roles of `access.fields` in E0.d: SCY-3.1, SCY-5.1.4 and `E0C-49`.

use menura_contracts::types::audit_record::OssieFormat;

use crate::extensions::security::fixture::{
    MODEL, NOTES_DATA, ORDERS_DATA, conformance_model, with_access, with_notes,
};
use crate::extensions::{DEFAULT_NAMESPACE, validate_extensions};
use crate::read::read_document;
use crate::references::security::resolve_security_references;
use crate::structure::validate_structure;

/// The code and JSON Pointer of every `E0C-49` diagnostic of `model`.
fn unresolved(model: &str) -> Vec<(&'static str, String)> {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure =
        validate_structure(&document, OssieFormat::Release, None).expect("the structure is valid");
    let extensions = validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
        .expect("the extensions are accepted");
    match resolve_security_references(&structure, &extensions) {
        Ok(()) => Vec::new(),
        Err(errors) => errors
            .iter()
            .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
            .collect(),
    }
}

fn orders() -> Vec<(&'static str, String)> {
    vec![("E0C-49", ORDERS_DATA.to_owned())]
}

#[test]
fn scy_5_1_4_the_fields_and_roles_of_the_valid_model_resolve() {
    assert!(unresolved(MODEL).is_empty());
}

#[test]
fn scy_5_1_4_e0c_49_a_missing_field() {
    assert_eq!(
        unresolved(conformance_model("e0-security-fields-001")),
        orders()
    );
    let several = r#"{"mode": "roles", "roles": ["admin"], "fields": [{"names": ["unit_cost", "margin"], "roles": ["admin"]}, {"names": ["markup"], "roles": ["admin"]}]}"#;
    assert_eq!(unresolved(&with_access(several)), orders());
}

#[test]
fn scy_5_1_1_field_names_are_declared_names() {
    let upper = r#"{"mode": "roles", "roles": ["admin"], "fields": [{"names": ["UNIT_COST"], "roles": ["admin"]}]}"#;
    assert!(unresolved(&with_access(upper)).is_empty());
    let delimited = r#"{"mode": "roles", "roles": ["admin"], "fields": [{"names": ["\"UNIT_COST\""], "roles": ["admin"]}]}"#;
    assert!(unresolved(&with_access(delimited)).is_empty());
    let lower = r#"{"mode": "roles", "roles": ["admin"], "fields": [{"names": ["\"unit_cost\""], "roles": ["admin"]}]}"#;
    assert_eq!(unresolved(&with_access(lower)), orders());
}

#[test]
fn scy_3_1_scy_5_1_4_e0c_49_a_role_without_access_to_the_dataset() {
    assert_eq!(
        unresolved(conformance_model("e0-security-field-role-001")),
        orders()
    );
    let authenticated = r#"{"mode": "authenticated", "fields": [{"names": ["unit_cost"], "roles": ["admin", "analyst"]}]}"#;
    assert!(unresolved(&with_access(authenticated)).is_empty());
    let notes = with_notes(
        r#""access": {"mode": "roles", "roles": ["analyst"], "fields": [{"names": ["body"], "roles": ["analyst", "admin"]}]}"#,
    );
    assert_eq!(unresolved(&notes), [("E0C-49", NOTES_DATA.to_owned())]);
}

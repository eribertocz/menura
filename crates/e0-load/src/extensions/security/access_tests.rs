//! The `access` of a dataset in E0.c: SCY-3, SCY-5, SCY-5.1.1, SCY-5.1.3, `E0C-02`, `E0C-03`,
//! `E0C-41` to `E0C-44` and `E0C-47`.

use semantic_compiler_security::AccessMode;

use super::fixture::{NOTES, NOTES_DATA, with_access, with_notes};
use super::support::{access_codes, codes_at, validate};
use crate::pointer::JsonPointer;

#[test]
fn scy_5_1_3_e0c_41_a_missing_or_unknown_mode() {
    assert_eq!(access_codes(r#"{"roles": ["analyst"]}"#), ["E0C-41"]);
    assert_eq!(access_codes(r#"{"mode": "public"}"#), ["E0C-41"]);
    assert_eq!(access_codes(r#"{"mode": 1}"#), ["E0C-41"]);
    assert_eq!(
        access_codes(r#"{"mode": "ROLES", "roles": ["analyst"]}"#),
        ["E0C-41"]
    );
}

#[test]
fn scy_5_1_3_e0c_42_mode_roles_requires_a_non_empty_role_list() {
    assert_eq!(access_codes(r#"{"mode": "roles"}"#), ["E0C-42"]);
    assert_eq!(
        access_codes(r#"{"mode": "roles", "roles": []}"#),
        ["E0C-42"]
    );
}

#[test]
fn scy_5_1_3_e0c_43_roles_and_fields_outside_their_modes() {
    assert_eq!(
        access_codes(r#"{"mode": "authenticated", "roles": ["analyst"]}"#),
        ["E0C-43"]
    );
    let fields = r#""fields": [{"names": ["unit_cost"], "roles": ["admin"]}]"#;
    assert!(
        validate(&with_access(&format!(
            r#"{{"mode": "authenticated", {fields}}}"#
        )))
        .is_ok()
    );
    let summary = with_notes(&format!(
        r#""access": {{"mode": "internal", "roles": ["admin"], {fields}}}, "aggregate": {{}}"#
    ));
    assert_eq!(codes_at(&summary, NOTES_DATA), ["E0C-43"]);
}

#[test]
fn scy_5_e0c_44_internal_only_on_a_summary() {
    let internal = with_notes(r#""access": {"mode": "internal"}"#);
    assert_eq!(codes_at(&internal, NOTES_DATA), ["E0C-44"]);
    let summary = with_notes(r#""access": {"mode": "internal"}, "aggregate": {}"#);
    let security = validate(&summary)
        .expect("the extensions are accepted")
        .security;
    let notes = JsonPointer::from_text(NOTES);
    assert_eq!(
        security
            .access(&notes)
            .map(|declared| &declared.access.mode),
        Some(&AccessMode::Internal)
    );
}

#[test]
fn scy_5_1_3_e0c_02_e0c_03_the_shape_of_access() {
    assert_eq!(access_codes(r#""roles""#), ["E0C-03"]);
    assert_eq!(
        access_codes(r#"{"mode": "authenticated", "owner": "sales"}"#),
        ["E0C-02"]
    );
    assert_eq!(
        access_codes(r#"{"mode": "roles", "roles": "analyst"}"#),
        ["E0C-03"]
    );
    assert_eq!(
        access_codes(r#"{"mode": "roles", "roles": [1]}"#),
        ["E0C-03"]
    );
    for fields in [
        r#"{"names": ["unit_cost"], "roles": ["admin"]}"#,
        r#"[{"names": ["unit_cost"]}]"#,
        r#"[{"roles": ["admin"]}]"#,
        r#"[{"names": [], "roles": ["admin"]}]"#,
        r#"[{"names": ["unit_cost"], "roles": []}]"#,
        r#"[{"names": "unit_cost", "roles": ["admin"]}]"#,
        r#"["unit_cost"]"#,
    ] {
        let access = format!(r#"{{"mode": "roles", "roles": ["admin"], "fields": {fields}}}"#);
        assert_eq!(access_codes(&access), ["E0C-03"], "{fields}");
    }
    let unknown = r#"{"mode": "roles", "roles": ["admin"], "fields": [{"names": ["unit_cost"], "roles": ["admin"], "mask": true}]}"#;
    assert_eq!(access_codes(unknown), ["E0C-02"]);
}

#[test]
fn scy_5_1_1_scy_5_1_3_e0c_47_undeclared_roles_of_access() {
    assert_eq!(
        access_codes(r#"{"mode": "roles", "roles": ["Analyst"]}"#),
        ["E0C-47"]
    );
    let fields = r#"{"mode": "roles", "roles": ["admin"], "fields": [{"names": ["unit_cost"], "roles": ["auditor"]}]}"#;
    assert_eq!(access_codes(fields), ["E0C-47"]);
}

#[test]
fn scy_5_1_3_e0c_03_repeats_in_access_lists() {
    for access in [
        r#"{"mode": "roles", "roles": ["analyst", "analyst"]}"#,
        r#"{"mode": "roles", "roles": ["admin"], "fields": [{"names": ["unit_cost", "unit_cost"], "roles": ["admin"]}]}"#,
        r#"{"mode": "roles", "roles": ["admin"], "fields": [{"names": ["unit_cost"], "roles": ["admin", "admin"]}]}"#,
    ] {
        assert_eq!(access_codes(access), ["E0C-03"], "{access}");
    }
    let apart = r#"{"mode": "roles", "roles": ["admin"], "fields": [{"names": ["unit_cost"], "roles": ["admin"]}, {"names": ["unit_cost"], "roles": ["admin"]}]}"#;
    assert!(validate(&with_access(apart)).is_ok());
}

//! SEC-3.6: the context fingerprint, over the declared roles and attributes only.

mod support;

use menura_contracts::Sha256Hash;
use menura_model::internal_model::{ScalarType, SecurityContext};
use menura_security::admit_security_context;
use support::{decimal, declarations, document, document_named, list, scalar};

fn model() -> SecurityContext {
    declarations(
        &["admin", "analyst"],
        &[
            ("regions", list(ScalarType::Text)),
            ("areas", list(ScalarType::Integer)),
            ("limits", list(decimal(5, 2))),
            ("days", list(ScalarType::Date)),
            ("flags", list(ScalarType::Boolean)),
            ("area_id", scalar(ScalarType::Integer)),
        ],
    )
}

// `cfg(test)` marks the helper as test code, where `unwrap` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
fn fingerprint(text: &str) -> Sha256Hash {
    admit_security_context(text, Some(&model()))
        .unwrap()
        .fingerprint()
}

fn fingerprint_of(roles: &str, attributes: &str) -> Sha256Hash {
    fingerprint(&document(roles, attributes))
}

#[test]
fn sec_3_6_fixed_hash_of_an_example_context() {
    let hash = fingerprint_of(
        r#"["analyst"]"#,
        r#"{"regions": ["SOUTH", "NORTH"], "area_id": 7}"#,
    );
    // SHA-256 of {"attributes":{"area_id":7,"regions":["NORTH","SOUTH"]},"roles":["analyst"]}
    assert_eq!(
        hash.as_str(),
        "5b54ef0feedf2f6ab04e03752a9c91e3a9842c6f70eb5b554caa1696b99e6aec"
    );
}

#[test]
fn sec_3_6_subject_is_excluded() {
    let first = document_named("menura.security_context", "alice", r#"["admin"]"#, "{}");
    let second = document_named("menura.security_context", "bob", r#"["admin"]"#, "{}");
    assert_eq!(fingerprint(&first), fingerprint(&second));
}

#[test]
fn sec_3_6_role_order_does_not_matter() {
    assert_eq!(
        fingerprint_of(r#"["analyst", "admin"]"#, "{}"),
        fingerprint_of(r#"["admin", "analyst"]"#, "{}")
    );
}

#[test]
fn sec_3_6_list_order_and_duplicates_do_not_matter() {
    assert_eq!(
        fingerprint_of(
            "[]",
            r#"{"regions": ["b", "a", "b"], "flags": [true, false, true]}"#
        ),
        fingerprint_of("[]", r#"{"regions": ["a", "b"], "flags": [false, true]}"#)
    );
}

#[test]
fn sec_3_6_lists_sort_by_value_in_their_declared_type() {
    assert_eq!(
        fingerprint_of("[]", r#"{"areas": [10, 9]}"#),
        fingerprint_of("[]", r#"{"areas": [9, 10]}"#)
    );
    assert_eq!(
        fingerprint_of("[]", r#"{"areas": ["9007199254740993", -1, 10]}"#),
        fingerprint_of("[]", r#"{"areas": [-1, 10, "9007199254740993"]}"#)
    );
    assert_eq!(
        fingerprint_of("[]", r#"{"limits": ["10.00", "9.50", "-1.00"]}"#),
        fingerprint_of("[]", r#"{"limits": ["-1.00", "9.50", "10.00"]}"#)
    );
    assert_eq!(
        fingerprint_of("[]", r#"{"days": ["2026-01-02", "2025-12-31"]}"#),
        fingerprint_of("[]", r#"{"days": ["2025-12-31", "2026-01-02"]}"#)
    );
}

#[test]
fn sec_3_6_integer_lists_are_numeric_in_the_hashed_document() {
    let expected = Sha256Hash::try_from(
        "6c44ed7f4033e9c588447b07d1bbabd08d7db554a37d4c6127d883a59b06bb6b".to_owned(),
    )
    .unwrap();
    // SHA-256 of {"attributes":{"areas":[9,10]},"roles":[]}
    assert_eq!(fingerprint_of("[]", r#"{"areas": [10, 9]}"#), expected);
}

#[test]
fn sec_3_6_undeclared_roles_and_attributes_are_excluded() {
    assert_eq!(
        fingerprint_of(r#"["admin", "viewer"]"#, r#"{"area_id": 1, "other": "x"}"#),
        fingerprint_of(r#"["admin"]"#, r#"{"area_id": 1}"#)
    );
}

#[test]
fn sec_3_6_a_declared_role_or_value_changes_the_fingerprint() {
    let base = fingerprint_of(r#"["admin"]"#, r#"{"area_id": 1}"#);
    assert_ne!(base, fingerprint_of(r#"["analyst"]"#, r#"{"area_id": 1}"#));
    assert_ne!(base, fingerprint_of(r#"["admin"]"#, r#"{"area_id": 2}"#));
    assert_ne!(base, fingerprint_of(r#"["admin"]"#, "{}"));
}

#[test]
fn sec_3_6_is_64_lowercase_hex_digits() {
    let hash = fingerprint_of("[]", "{}");
    assert_eq!(hash.as_str().len(), 64);
    assert!(
        hash.as_str()
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    );
}

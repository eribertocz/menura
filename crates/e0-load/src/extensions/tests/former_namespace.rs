//! Entries under the former namespace: CON-0.5 and W0C-02.

use super::{MODEL, ORDERS_DATA, accepted, errors, on_orders, pair, validate};
use crate::extensions::DEFAULT_NAMESPACE;
use crate::extensions::output::Extensions;

/// The code and JSON Pointer of every warning of accepted extensions.
fn warnings(extensions: &Extensions) -> Vec<(&'static str, String)> {
    extensions
        .warnings
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect()
}

#[test]
fn con_0_5_e0c_1_w0c_02_an_entry_under_the_former_namespace_is_ours() {
    let former = on_orders(
        r#"{"namespace": "semantic_compiler", "version": 1, "access": {"mode": "authenticated"}}"#,
    );
    let extensions = accepted(former.as_bytes());
    assert_eq!(extensions.entries.len(), 1);
    assert_eq!(warnings(&extensions), [pair("W0C-02", ORDERS_DATA)]);
    assert!(extensions.warnings[0].code.is_warning());
    let model = conformance_model!("e0-extensions-former-namespace-001");
    assert_eq!(
        warnings(&accepted(model)),
        [pair(
            "W0C-02",
            &format!("{MODEL}/datasets/0/custom_extensions/0/data")
        )]
    );
}

#[test]
fn con_0_5_w0c_02_is_reported_whatever_else_the_entry_reports() {
    let former = on_orders(r#"{"namespace": "semantic_compiler", "version": 2}"#);
    let failure = validate(former.as_bytes(), DEFAULT_NAMESPACE).expect_err("rejected");
    let warnings: Vec<_> = failure
        .warnings
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect();
    assert_eq!(warnings, [pair("W0C-02", ORDERS_DATA)]);
    assert_eq!(
        errors(former.as_bytes(), DEFAULT_NAMESPACE),
        [pair("E0C-01", ORDERS_DATA)]
    );
}

#[test]
fn con_0_5_mdl_2_1_e0c_04_entries_under_both_namespaces_are_two_entries_of_ours() {
    let model = conformance_model!("e0-extensions-two-namespaces-001");
    let failure = validate(model, DEFAULT_NAMESPACE).expect_err("rejected");
    let warnings: Vec<_> = failure
        .warnings
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect();
    assert_eq!(
        warnings,
        [pair(
            "W0C-02",
            &format!("{MODEL}/datasets/0/custom_extensions/1/data")
        )]
    );
    assert_eq!(
        errors(model, DEFAULT_NAMESPACE),
        [pair(
            "E0C-04",
            &format!("{MODEL}/datasets/0/custom_extensions/1")
        )]
    );
}

#[test]
fn con_0_5_no_w0c_02_when_the_former_namespace_is_the_configured_one() {
    let former = on_orders(
        r#"{"namespace": "semantic_compiler", "version": 1, "access": {"mode": "authenticated"}}"#,
    );
    let extensions = validate(former.as_bytes(), "semantic_compiler").expect("accepted");
    assert_eq!(extensions.entries.len(), 1);
    assert!(extensions.warnings.is_empty());
    let acme = validate(former.as_bytes(), "acme").expect("accepted");
    assert_eq!(acme.entries.len(), 1);
    assert_eq!(warnings(&acme), [pair("W0C-02", ORDERS_DATA)]);
}

#[test]
fn con_0_5_e0c_1_3_no_w0c_02_when_a_repeated_namespace_matches_the_configured_one() {
    let repeated =
        on_orders(r#"{"namespace": "semantic_compiler", "namespace": "menura", "version": 1}"#);
    let failure = validate(repeated.as_bytes(), DEFAULT_NAMESPACE).expect_err("rejected");
    assert!(failure.warnings.is_empty());
    let former_only =
        on_orders(r#"{"namespace": "acme", "namespace": "semantic_compiler", "version": 1}"#);
    let failure = validate(former_only.as_bytes(), DEFAULT_NAMESPACE).expect_err("rejected");
    assert_eq!(failure.warnings.len(), 1);
    assert_eq!(failure.warnings[0].code.as_str(), "W0C-02");
}

//! The common extension rules: E0C-1.1 to E0C-1.3, E0C-2.1 to E0C-2.7, E0C-01, E0C-02, E0C-04 to
//! E0C-06, W0C-01, W0C-02,
//! MDL-2.1 to MDL-2.3, MDL-2.5 and CON-0.5.
//!
//! The model documents of the `e0-extensions-*` conformance cases are read here at compile
//! time, so the rules are checked against the same files the conformance suite uses.

use menura_contracts::types::audit_record::OssieFormat;
use serde_json::json;

use super::output::{Extensions, ExtensionsFailure, ObjectKind};
use super::{DEFAULT_NAMESPACE, validate_extensions};
use crate::read::read_document;
use crate::structure::validate_structure;

macro_rules! conformance_model {
    ($name:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../conformance/models/",
            $name,
            "/model.yaml"
        ))
    };
}

fn validate(bytes: &[u8], namespace: &str) -> Result<Extensions, ExtensionsFailure> {
    let document = read_document(bytes).expect("the document is read");
    let structure =
        validate_structure(&document, OssieFormat::Release, None).expect("the structure is valid");
    validate_extensions(&document, &structure.semantic_model, namespace)
}

fn accepted(bytes: &[u8]) -> Extensions {
    validate(bytes, DEFAULT_NAMESPACE).expect("the extensions are accepted")
}

/// The code and JSON Pointer of every rejecting diagnostic.
fn errors(bytes: &[u8], namespace: &str) -> Vec<(&'static str, String)> {
    let failure = validate(bytes, namespace).expect_err("the extensions are rejected");
    failure
        .errors
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect()
}

fn pair(code: &'static str, pointer: &str) -> (&'static str, String) {
    (code, pointer.to_owned())
}

/// A minimal valid document whose orders dataset carries one `COMMON` entry with `data`.
fn on_orders(data: &str) -> String {
    on_orders_entries(&format!(
        "          - vendor_name: COMMON\n            data: '{data}'\n"
    ))
}

/// A minimal valid document whose orders dataset carries the given `custom_extensions` lines.
fn on_orders_entries(lines: &str) -> String {
    format!(
        "version: \"0.1.1\"\nsemantic_model:\n  - name: shop\n    datasets:\n      - name: orders\n        source: shop.orders\n        custom_extensions:\n{lines}"
    )
}

const MODEL: &str = "/semantic_model/0";
const ORDERS_DATA: &str = "/semantic_model/0/datasets/0/custom_extensions/0/data";

/// The object type, object pointer and extension keys of every entry of ours.
fn entry_summary(extensions: &Extensions) -> Vec<(ObjectKind, String, Vec<String>)> {
    extensions
        .entries
        .iter()
        .map(|entry| {
            (
                entry.object,
                entry.object_pointer.to_string(),
                entry.extensions.keys().cloned().collect(),
            )
        })
        .collect()
}

/// The entries of ours of every `e0-extensions-*` model built from the sample model.
fn sample_entries() -> Vec<(ObjectKind, String, Vec<String>)> {
    vec![
        (
            ObjectKind::Model,
            MODEL.to_owned(),
            vec!["security_context".to_owned()],
        ),
        (
            ObjectKind::Dataset,
            format!("{MODEL}/datasets/0"),
            vec!["access".to_owned()],
        ),
        (
            ObjectKind::Dataset,
            format!("{MODEL}/datasets/1"),
            vec!["access".to_owned()],
        ),
        (
            ObjectKind::Metric,
            format!("{MODEL}/metrics/1"),
            vec!["additivity".to_owned()],
        ),
    ]
}

#[test]
fn e0c_1_1_mdl_2_1_mdl_2_5_other_entries_are_kept_and_never_interpreted() {
    let extensions = accepted(conformance_model!("e0-extensions-other-namespace-001"));
    assert_eq!(entry_summary(&extensions), sample_entries());
    assert!(extensions.warnings.is_empty());
    let not_an_object = on_orders("[1, 2]");
    assert!(accepted(not_an_object.as_bytes()).entries.is_empty());
    let no_namespace = on_orders(r#"{"version": 1, "owner": 5}"#);
    assert!(accepted(no_namespace.as_bytes()).entries.is_empty());
    let not_text = on_orders(r#"{"namespace": 5, "version": 0}"#);
    assert!(accepted(not_text.as_bytes()).entries.is_empty());
}

#[test]
fn e0c_1_2_w0c_01_common_entry_that_is_not_json() {
    let extensions = accepted(conformance_model!("e0-extensions-not-json-001"));
    assert_eq!(entry_summary(&extensions), sample_entries());
    let warnings: Vec<_> = extensions
        .warnings
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect();
    assert_eq!(
        warnings,
        [pair(
            "W0C-01",
            &format!("{MODEL}/metrics/0/custom_extensions/0/data")
        )]
    );
    assert!(
        extensions
            .warnings
            .iter()
            .all(|warning| warning.code.is_warning())
    );
    let other_vendor =
        on_orders_entries("          - vendor_name: DBT\n            data: 'not json'\n");
    assert!(accepted(other_vendor.as_bytes()).warnings.is_empty());
}

#[test]
fn e0c_2_1_mdl_2_2_e0c_01_version_missing_or_not_supported() {
    let found = errors(
        conformance_model!("e0-extensions-unsupported-version-001"),
        DEFAULT_NAMESPACE,
    );
    assert_eq!(
        found,
        [pair(
            "E0C-01",
            &format!("{MODEL}/metrics/1/custom_extensions/0/data")
        )]
    );
    for data in [
        r#"{"namespace": "menura", "access": {}}"#,
        r#"{"namespace": "menura", "version": 0, "owner": 1}"#,
        r#"{"namespace": "menura", "version": -1}"#,
    ] {
        let document = on_orders(data);
        assert_eq!(
            errors(document.as_bytes(), DEFAULT_NAMESPACE),
            [pair("E0C-01", ORDERS_DATA)],
            "{data}"
        );
    }
}

#[test]
fn e0c_2_1_e0c_01_version_of_another_json_type() {
    let found = errors(
        conformance_model!("e0-extensions-wrong-type-001"),
        DEFAULT_NAMESPACE,
    );
    assert_eq!(
        found,
        [pair(
            "E0C-01",
            &format!("{MODEL}/datasets/0/custom_extensions/0/data")
        )]
    );
    for version in ["1.0", "1e0", "null", "true", "[1]", "{}"] {
        let document = on_orders(&format!(
            r#"{{"namespace": "menura", "version": {version}}}"#
        ));
        assert_eq!(
            errors(document.as_bytes(), DEFAULT_NAMESPACE),
            [pair("E0C-01", ORDERS_DATA)],
            "{version}"
        );
    }
}

#[test]
fn e0c_1_3_a_repeated_namespace_is_ours_when_any_value_matches() {
    let found = errors(
        conformance_model!("e0-extensions-repeated-namespace-001"),
        DEFAULT_NAMESPACE,
    );
    assert_eq!(
        found,
        [pair(
            "E0C-06",
            &format!("{MODEL}/datasets/0/custom_extensions/0/data")
        )]
    );
    for data in [
        r#"{"namespace": "acme", "namespace": "menura", "version": 1}"#,
        r#"{"namespace": "menura", "namespace": "acme", "version": 1}"#,
        r#"{"namespace": "menura", "namespace": "menura", "version": 1}"#,
    ] {
        let document = on_orders(data);
        assert_eq!(
            errors(document.as_bytes(), DEFAULT_NAMESPACE),
            [pair("E0C-06", ORDERS_DATA)],
            "{data}"
        );
    }
    let neither = on_orders(r#"{"namespace": "acme", "namespace": "other", "version": 1}"#);
    assert!(accepted(neither.as_bytes()).entries.is_empty());
}

#[test]
fn e0c_2_7_e0c_06_repeated_member_inside_data() {
    let found = errors(
        conformance_model!("e0-extensions-repeated-member-001"),
        DEFAULT_NAMESPACE,
    );
    assert_eq!(
        found,
        [pair(
            "E0C-06",
            &format!("{MODEL}/datasets/0/custom_extensions/0/data")
        )]
    );
    for data in [
        r#"{"namespace": "menura", "version": 2, "version": 1}"#,
        r#"{"namespace": "menura", "version": 1, "row_access": [{"a": 1, "b": {"c": 1, "c": 2}}]}"#,
    ] {
        let document = on_orders(data);
        assert_eq!(
            errors(document.as_bytes(), DEFAULT_NAMESPACE),
            [pair("E0C-06", ORDERS_DATA)],
            "{data}"
        );
    }
    let failure = validate(
        on_orders(r#"{"namespace": "menura", "version": 1, "x": {"k": 1, "k": 2}, "k": 0}"#)
            .as_bytes(),
        DEFAULT_NAMESPACE,
    )
    .expect_err("rejected");
    assert!(
        failure.errors[0].message.contains("`k`"),
        "{}",
        failure.errors[0].message
    );
    let foreign = on_orders(r#"{"namespace": "acme", "version": 1, "version": 2}"#);
    assert!(accepted(foreign.as_bytes()).entries.is_empty());
    let same_names_apart = on_orders(
        r#"{"namespace": "menura", "version": 1, "row_access": [{"name": "a", "condition": "TRUE"}, {"name": "b", "condition": "TRUE"}]}"#,
    );
    assert_eq!(accepted(same_names_apart.as_bytes()).entries.len(), 1);
}

#[test]
fn e0c_2_2_mdl_2_1_e0c_04_more_than_one_entry_of_ours() {
    let found = errors(
        conformance_model!("e0-extensions-two-entries-001"),
        DEFAULT_NAMESPACE,
    );
    assert_eq!(
        found,
        [pair(
            "E0C-04",
            &format!("{MODEL}/datasets/0/custom_extensions/1")
        )]
    );
    let foreign_between = on_orders_entries(concat!(
        "          - vendor_name: COMMON\n            data: '{\"namespace\": \"menura\", \"version\": 1}'\n",
        "          - vendor_name: COMMON\n            data: '{\"namespace\": \"acme\", \"version\": 1}'\n",
        "          - vendor_name: SNOWFLAKE\n            data: '{\"namespace\": \"menura\", \"version\": 1}'\n",
    ));
    assert_eq!(accepted(foreign_between.as_bytes()).entries.len(), 1);
}

#[test]
fn e0c_2_3_e0c_05_extension_key_on_the_wrong_object_type() {
    let found = errors(
        conformance_model!("e0-extensions-wrong-object-001"),
        DEFAULT_NAMESPACE,
    );
    assert_eq!(
        found,
        [pair(
            "E0C-05",
            &format!("{MODEL}/datasets/0/fields/2/custom_extensions/0/data")
        )]
    );
    let model_key = on_orders(r#"{"namespace": "menura", "version": 1, "security_context": {}}"#);
    assert_eq!(
        errors(model_key.as_bytes(), DEFAULT_NAMESPACE),
        [pair("E0C-05", ORDERS_DATA)]
    );
}

#[test]
fn e0c_2_4_e0c_02_unknown_key_in_an_entry_of_ours() {
    let found = errors(
        conformance_model!("e0-extensions-unknown-key-001"),
        DEFAULT_NAMESPACE,
    );
    assert_eq!(
        found,
        [pair(
            "E0C-02",
            &format!("{MODEL}/datasets/0/custom_extensions/0/data")
        )]
    );
    let both = on_orders(
        r#"{"namespace": "menura", "version": 1, "owner": 1, "additivity": {}, "team": 2}"#,
    );
    let failure = validate(both.as_bytes(), DEFAULT_NAMESPACE).expect_err("rejected");
    let codes: Vec<_> = failure
        .errors
        .iter()
        .map(|error| error.code.as_str())
        .collect();
    assert_eq!(codes, ["E0C-02", "E0C-05"]);
    assert!(failure.errors[0].message.contains("`owner`, `team`"));
}

#[test]
fn e0c_2_5_extension_keys_are_handed_over_uninterpreted() {
    let extensions = accepted(conformance_model!("sample"));
    assert_eq!(entry_summary(&extensions), sample_entries());
    let additivity = &extensions.entries[3];
    assert_eq!(
        additivity.data_pointer.to_string(),
        format!("{MODEL}/metrics/1/custom_extensions/0/data")
    );
    assert_eq!(
        additivity.extensions["additivity"],
        json!({"type": "semi_additive", "over": "balances.branch", "entity": ["balances.customer_id"], "aggregation": "max", "nulls": "error"})
    );
    let unchecked =
        on_orders(r#"{"namespace": "menura", "version": 1, "access": {"mode": "authenticated"}}"#);
    assert_eq!(
        accepted(unchecked.as_bytes()).entries[0].extensions["access"],
        json!({"mode": "authenticated"})
    );
}

#[test]
fn mdl_2_3_the_namespace_is_configurable() {
    let model = conformance_model!("e0-extensions-other-namespace-001");
    assert_eq!(
        errors(model, "acme"),
        [pair(
            "E0C-01",
            &format!("{MODEL}/datasets/0/custom_extensions/1/data")
        )]
    );
    let acme = on_orders(r#"{"namespace": "acme", "version": 1, "row_access": []}"#);
    let extensions = validate(acme.as_bytes(), "acme").expect("accepted");
    assert_eq!(extensions.entries.len(), 1);
    assert!(accepted(acme.as_bytes()).entries.is_empty());
}

// Declared after `conformance_model!`, so the submodules can use it.
mod diagnostic_order;
mod former_namespace;

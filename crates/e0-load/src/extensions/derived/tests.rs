//! The derived-metric extension in E0.c: DER-3, DER-5.1.1, `E0C-02`, `E0C-03`, `E0C-05` and
//! `E0C-36`.
//!
//! The model documents of the `e0-derived*` conformance cases are read here at compile time
//! (see [`super::fixture`]), so the rules are checked against the files the conformance suite
//! uses.

use menura_contracts::types::audit_record::OssieFormat;

use super::DivisionByZero;
use super::fixture::{
    BILLED_PER_CUSTOMER, COLLECTION_RATE, MODEL, UNCOLLECTED_RATE, conformance_model, data, with,
    with_uncollected_derived,
};
use crate::extensions::output::{Extensions, ExtensionsFailure};
use crate::extensions::{DEFAULT_NAMESPACE, ExtensionCode, validate_extensions};
use crate::pointer::JsonPointer;
use crate::read::read_document;
use crate::structure::validate_structure;

fn validate(model: &str) -> Result<Extensions, ExtensionsFailure> {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure =
        validate_structure(&document, OssieFormat::Release, None).expect("the structure is valid");
    validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
}

/// The code and JSON Pointer of every rejecting diagnostic of `model`.
fn rejected(model: &str) -> Vec<(&'static str, String)> {
    validate(model)
        .expect_err("the extensions are rejected")
        .errors
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect()
}

fn pair(code: &'static str, pointer: &str) -> (&'static str, String) {
    (code, pointer.to_owned())
}

#[test]
fn der_3_der_5_1_1_the_declarations_of_the_example_are_read() {
    let extensions = validate(MODEL).expect("the extensions are accepted");
    let found: Vec<(String, Option<DivisionByZero>)> = extensions
        .derived
        .iter()
        .map(|declaration| {
            assert_eq!(
                declaration.data.to_string(),
                data(declaration.metric.as_str())
            );
            (
                declaration.metric.to_string(),
                declaration.on_division_by_zero,
            )
        })
        .collect();
    assert_eq!(
        found,
        [
            (COLLECTION_RATE.to_owned(), Some(DivisionByZero::Null)),
            (UNCOLLECTED_RATE.to_owned(), None),
            (BILLED_PER_CUSTOMER.to_owned(), Some(DivisionByZero::Error)),
        ]
    );
    let pointer = JsonPointer::root()
        .member("semantic_model")
        .item(0)
        .member("metrics")
        .item(2);
    assert!(extensions.derived_declaration(&pointer).is_some());
    assert!(!extensions.is_context_metric(&pointer));
}

#[test]
fn der_5_1_1_e0c_03_a_value_that_is_not_listed_or_not_an_object() {
    assert_eq!(
        rejected(conformance_model("e0-derived-wrong-value-001")),
        [pair("E0C-03", &data(COLLECTION_RATE))]
    );
    for derived in [
        r#""yes""#,
        "[]",
        "null",
        r#"{"on_division_by_zero": "NULL"}"#,
    ] {
        assert_eq!(
            rejected(&with_uncollected_derived(derived)),
            [pair("E0C-03", &data(UNCOLLECTED_RATE))],
            "{derived}"
        );
    }
}

#[test]
fn der_5_1_1_e0c_02_an_unknown_key() {
    assert_eq!(
        rejected(conformance_model("e0-derived-unknown-key-001")),
        [pair("E0C-02", &data(UNCOLLECTED_RATE))]
    );
    assert_eq!(
        rejected(&with_uncollected_derived(
            r#"{"rounding": 2, "on_division_by_zero": 0}"#
        )),
        [
            pair("E0C-02", &data(UNCOLLECTED_RATE)),
            pair("E0C-03", &data(UNCOLLECTED_RATE)),
        ]
    );
}

#[test]
fn der_5_1_1_e0c_36_a_derived_metric_with_additivity() {
    assert_eq!(
        rejected(conformance_model("e0-derived-additivity-001")),
        [pair("E0C-36", &data(BILLED_PER_CUSTOMER))]
    );
    assert_eq!(ExtensionCode::DerivedAdditivity.as_str(), "E0C-36");
}

#[test]
fn der_5_1_1_e0c_05_derived_is_valid_on_metrics_only() {
    let model = with(
        "      - name: customers\n        source: shop.main.customers\n",
        "      - name: customers\n        source: shop.main.customers\n        custom_extensions:\n          \
         - vendor_name: COMMON\n            data: '{\"namespace\": \"menura\", \"version\": 1, \"derived\": {}}'\n",
    );
    assert_eq!(
        rejected(&model),
        [pair(
            "E0C-05",
            "/semantic_model/0/datasets/1/custom_extensions/0/data"
        )]
    );
}

#[test]
fn e0c_37_is_reported_as_w0g_01() {
    let code = ExtensionCode::AggregateOutsideDerived;
    assert_eq!((code.as_str(), code.load_code()), ("E0C-37", "W0G-01"));
    assert!(code.is_warning());
    assert!(!ExtensionCode::DependencyCycle.is_warning());
    assert_eq!(ExtensionCode::DependencyCycle.load_code(), "E0C-34");
}

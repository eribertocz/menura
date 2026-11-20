//! The inputs of derived metrics in E0.d: DER-2.1, DER-2.2, DER-5.1.2, `E0C-32`, `E0C-33` and
//! `E0C-37`.

use menura_contracts::types::audit_record::OssieFormat;

use crate::extensions::derived::DivisionByZero;
use crate::extensions::derived::fixture::{
    BILLED, BILLED_PER_CUSTOMER, COLLECTED, COLLECTION_RATE, CUSTOMER_COUNT, MODEL,
    UNCOLLECTED_RATE, conformance_model, data, with, with_uncollected_rate,
};
use crate::extensions::{DEFAULT_NAMESPACE, validate_extensions};
use crate::read::read_document;
use crate::references::derived::{
    DerivedMetrics, DerivedOrigin, DerivedRejection, resolve_derived_references,
};
use crate::structure::validate_structure;

fn resolve_in(model: &str, format: OssieFormat) -> Result<DerivedMetrics, DerivedRejection> {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure = validate_structure(&document, format, None).expect("the structure is valid");
    let extensions = validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
        .expect("the extensions are accepted");
    resolve_derived_references(&document, &structure, &extensions)
}

fn resolved(model: &str) -> DerivedMetrics {
    resolve_in(model, OssieFormat::Release).expect("the inputs resolve")
}

fn rejected(model: &str) -> Vec<(&'static str, String)> {
    resolve_in(model, OssieFormat::Release)
        .expect_err("the model is rejected")
        .errors
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect()
}

/// The pointers of the metrics each derived metric reads, by derived metric.
fn inputs(derived: &DerivedMetrics) -> Vec<(String, Vec<String>)> {
    derived
        .metrics
        .iter()
        .map(|metric| {
            let inputs = metric
                .inputs
                .iter()
                .map(|input| input.metric.to_string())
                .collect();
            (metric.pointer.to_string(), inputs)
        })
        .collect()
}

fn entry(metric: &str, inputs: &[&str]) -> (String, Vec<String>) {
    (
        metric.to_owned(),
        inputs.iter().map(|input| (*input).to_owned()).collect(),
    )
}

#[test]
fn der_2_1_der_5_1_2_the_inputs_of_the_example_resolve() {
    let derived = resolved(MODEL);
    assert_eq!(
        inputs(&derived),
        [
            entry(COLLECTION_RATE, &[COLLECTED, BILLED]),
            entry(UNCOLLECTED_RATE, &[COLLECTION_RATE]),
            entry(BILLED_PER_CUSTOMER, &[BILLED, CUSTOMER_COUNT]),
        ]
    );
    assert!(derived.warnings.is_empty() && derived.unusable.is_empty());
    let collection_rate = &derived.metrics[0];
    assert_eq!(collection_rate.name, "collection_rate");
    assert_eq!(
        collection_rate.origin,
        DerivedOrigin::Declared(Some(DivisionByZero::Null))
    );
    assert_eq!(
        collection_rate.diagnostic_pointer.to_string(),
        data(COLLECTION_RATE)
    );
}

#[test]
fn der_5_1_2_names_are_compared_after_normalization() {
    let derived = resolved(&with_uncollected_rate(
        r#"1 - AGG("COLLECTION_RATE") + AGG(Billed)"#,
    ));
    assert_eq!(
        inputs(&derived)[1],
        entry(UNCOLLECTED_RATE, &[COLLECTION_RATE, BILLED])
    );
    assert_eq!(
        rejected(&with_uncollected_rate(r#"1 - AGG("collection_rate")"#)),
        [("E0C-32", data(UNCOLLECTED_RATE))]
    );
}

#[test]
fn der_5_1_2_e0c_32_a_name_with_no_such_metric() {
    assert_eq!(
        rejected(conformance_model("e0-derived-unknown-metric-001")),
        [("E0C-32", data(COLLECTION_RATE))]
    );
    let rejection = resolve_in(
        &with_uncollected_rate("AGG(missing) + AGG(invoices.billed) + AGG(missing)"),
        OssieFormat::Release,
    )
    .expect_err("the model is rejected");
    assert_eq!(rejection.errors.len(), 1);
    assert!(
        rejection.errors[0]
            .message
            .contains("`AGG(missing)`, `AGG(invoices.billed)`"),
        "{}",
        rejection.errors[0].message
    );
}

#[test]
fn der_5_1_2_e0c_33_a_formula_with_no_metric() {
    assert_eq!(
        rejected(conformance_model("e0-derived-no-metric-001")),
        [("E0C-33", data(UNCOLLECTED_RATE))]
    );
}

#[test]
fn der_2_2_der_5_1_2_e0c_37_agg_outside_a_derived_metric() {
    let derived = resolved(conformance_model("e0-derived-unusable"));
    let billed_twice = "/semantic_model/0/metrics/1";
    assert_eq!(
        derived
            .unusable
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        [billed_twice]
    );
    let warnings: Vec<(&str, &str, String)> = derived
        .warnings
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.code.as_str(),
                diagnostic.code.load_code(),
                diagnostic.pointer.to_string(),
            )
        })
        .collect();
    assert_eq!(warnings, [("E0C-37", "W0G-01", billed_twice.to_owned())]);
    assert_eq!(
        inputs(&derived),
        [entry(
            "/semantic_model/0/metrics/2",
            &["/semantic_model/0/metrics/1", "/semantic_model/0/metrics/0"]
        )]
    );
}

#[test]
fn der_2_2_e0c_37_warnings_are_reported_with_a_rejection() {
    let model = with(
        "expression: {dialects: [{dialect: ANSI_SQL, expression: 'SUM(invoices.billed)'}]}",
        "expression: {dialects: [{dialect: ANSI_SQL, expression: 'AGG(collected) + 1'}]}",
    );
    let model = model.replacen("AGG(collected) / AGG(billed)", "AGG(nothing)", 1);
    let rejection = resolve_in(&model, OssieFormat::Release).expect_err("the model is rejected");
    assert_eq!(rejection.errors[0].code.as_str(), "E0C-32");
    assert_eq!(rejection.warnings[0].pointer.to_string(), BILLED);
}

#[test]
fn der_2_2_a_context_metric_is_left_to_its_extension() {
    let model = crate::extensions::context::fixture::MODEL;
    let derived = resolve_in(model, OssieFormat::Release).expect("the inputs resolve");
    assert!(derived.unusable.is_empty() && derived.warnings.is_empty());
    let derived: Vec<&str> = derived
        .metrics
        .iter()
        .map(|metric| metric.pointer.as_str())
        .collect();
    assert_eq!(
        derived,
        [crate::extensions::context::fixture::BILLED_CHANGE]
    );
}

#[test]
fn der_2_2_der_5_1_2_agg_of_a_dataset_metric_in_the_development_format() {
    let model = conformance_model("e0-derived-development");
    let derived = resolve_in(model, OssieFormat::Development).expect("the inputs resolve");
    assert_eq!(
        inputs(&derived),
        [entry(
            "/metrics/0",
            &["/datasets/0/metrics/0", "/datasets/0/metrics/1"]
        )]
    );
    assert_eq!(derived.metrics[0].origin, DerivedOrigin::Development);
    assert_eq!(
        derived.metrics[0].diagnostic_pointer.to_string(),
        "/metrics/0/expression"
    );
    let missing = model.replacen("AGG(orders.order_count)", "AGG(orders.missing)", 1);
    let rejection =
        resolve_in(&missing, OssieFormat::Development).expect_err("the model is rejected");
    assert_eq!(rejection.errors[0].code.as_str(), "E0C-32");
    let mixed = model.replacen("AGG(orders.order_count)", "AGG(order_count)", 1);
    let derived = resolve_in(&mixed, OssieFormat::Development).expect("the model loads");
    assert_eq!(
        derived
            .warnings
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        ["E0C-37"]
    );
}

#[test]
fn der_2_2_agg_of_a_dataset_metric_in_the_release_format_is_e0c_37() {
    let derived = resolved(&with(
        "expression: {dialects: [{dialect: ANSI_SQL, expression: 'SUM(invoices.billed)'}]}",
        "expression: {dialects: [{dialect: ANSI_SQL, expression: 'AGG(invoices.billed)'}]}",
    ));
    assert_eq!(derived.unusable[0].to_string(), BILLED);
}

//! The derived metrics in E0.f: DER-2.3, DER-2.4, DER-5.1.2 (`E0F-02`), DER-5.1.3, DER-5.1.4,
//! E0G-5.1, `E0C-30`, `E0C-31`, `E0C-34` and `E0C-35`.

use semantic_compiler_contracts::admit_catalog_schema;
use semantic_compiler_contracts::types::audit_record::OssieFormat;
use semantic_compiler_contracts::types::common::LogicalType;

use super::summary;
use crate::extensions::derived::fixture::{
    BILLED_PER_CUSTOMER, CATALOG, COLLECTION_RATE, DEVELOPMENT_CATALOG, MODEL, UNCOLLECTED_RATE,
    conformance_model, data, with, with_uncollected_rate,
};
use crate::extensions::summaries::SummaryDatasets;
use crate::extensions::{DEFAULT_NAMESPACE, ExtensionDiagnostic, validate_extensions};
use crate::read::read_document;
use crate::references::context::resolve_context_references;
use crate::references::derived::resolve_derived_references;
use crate::structure::validate_structure;
use crate::typing::derived::type_derived_metrics;
use crate::typing::value_type::double;
use crate::typing::{Typing, type_expressions};

/// What E0.f finds in `model`, read in `format`: the typing of the other elements, and the
/// typing of the derived metrics or the diagnostics that reject the model.
fn type_in(
    model: &str,
    catalog: &str,
    format: OssieFormat,
) -> (Typing, Result<Typing, Vec<ExtensionDiagnostic>>) {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure = validate_structure(&document, format, None).expect("the structure is valid");
    let catalog = admit_catalog_schema(catalog).expect("the catalog schema is admitted");
    let extensions = validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
        .expect("the extensions are accepted");
    let derived =
        resolve_derived_references(&document, &structure, &extensions).expect("the inputs resolve");
    let context =
        resolve_context_references(&document, &structure, &extensions).expect("the bases resolve");
    let typing = type_expressions(
        &document,
        &structure,
        &catalog,
        &derived,
        &SummaryDatasets::default(),
    );
    let derived_typing =
        type_derived_metrics(&document, &structure, &catalog, &typing, &derived, &context);
    (typing, derived_typing)
}

/// The typing of the derived metrics of the release-format `model`.
fn typed(model: &str) -> Typing {
    type_in(model, CATALOG, OssieFormat::Release)
        .1
        .expect("the formulas are accepted")
}

/// The code and JSON Pointer of every diagnostic that rejects `model`.
fn rejected(model: &str) -> Vec<(&'static str, String)> {
    let errors = type_in(model, CATALOG, OssieFormat::Release)
        .1
        .expect_err("the model is rejected");
    errors
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect()
}

/// The result type of the metric at `pointer` in `typing`, if it is typed.
fn type_of(typing: &Typing, pointer: &str) -> Option<LogicalType> {
    typing
        .elements
        .iter()
        .find(|element| element.pointer.as_str() == pointer)
        .and_then(|element| element.typed.logical().cloned())
}

#[test]
fn der_5_1_3_the_formulas_of_the_example_are_typed() {
    let (typing, derived) = type_in(MODEL, CATALOG, OssieFormat::Release);
    let derived = derived.expect("the formulas are accepted");
    assert!(derived.warnings.is_empty() && derived.unusable.is_empty());
    for metric in [COLLECTION_RATE, UNCOLLECTED_RATE, BILLED_PER_CUSTOMER] {
        assert_eq!(type_of(&derived, metric), Some(double()), "{metric}");
        assert_eq!(
            type_of(&typing, metric),
            None,
            "{metric} is typed by its own rules"
        );
    }
}

#[test]
fn der_5_1_3_agg_has_the_type_of_its_metric() {
    let derived = typed(&with_uncollected_rate("AGG(billed) + AGG(collected)"));
    assert_eq!(
        type_of(&derived, UNCOLLECTED_RATE),
        Some(LogicalType::Integer {
            bits: semantic_compiler_contracts::types::common::IntegerBits::OneHundredTwentyEight,
            signed: true
        })
    );
    let derived = typed(&with_uncollected_rate(
        "CASE WHEN AGG(collection_rate) > 0 THEN AGG(collection_rate) END",
    ));
    assert_eq!(type_of(&derived, UNCOLLECTED_RATE), Some(double()));
}

#[test]
fn der_5_1_3_a_typing_problem_makes_the_derived_metric_unusable() {
    let derived = typed(&with_uncollected_rate("AGG(collection_rate) + 'x'"));
    assert_eq!(
        summary(&derived.warnings),
        [("E0F-03", format!("{UNCOLLECTED_RATE}/expression"))]
    );
    assert_eq!(derived.unusable[0].pointer.as_str(), UNCOLLECTED_RATE);
    assert_eq!(type_of(&derived, UNCOLLECTED_RATE), None);
}

#[test]
fn der_2_3_der_5_1_3_e0c_30_an_aggregate_function_in_the_formula() {
    assert_eq!(
        rejected(conformance_model("e0-derived-aggregate-001")),
        [("E0C-30", data(UNCOLLECTED_RATE))]
    );
    assert_eq!(
        rejected(&with_uncollected_rate(
            "SUM(invoices.billed) - AGG(collection_rate)"
        )),
        [
            ("E0C-30", data(UNCOLLECTED_RATE)),
            ("E0C-31", data(UNCOLLECTED_RATE)),
        ]
    );
}

#[test]
fn der_2_3_der_5_1_3_e0c_31_a_field_reference_in_the_formula() {
    assert_eq!(
        rejected(conformance_model("e0-derived-field-reference-001")),
        [("E0C-31", data(UNCOLLECTED_RATE))]
    );
}

#[test]
fn der_3_der_5_1_3_e0c_35_a_formula_that_divides_without_its_rule() {
    assert_eq!(
        rejected(conformance_model("e0-derived-division-001")),
        [("E0C-35", data(BILLED_PER_CUSTOMER))]
    );
    assert_eq!(
        rejected(&with_uncollected_rate("1 % AGG(collection_rate)")),
        [("E0C-35", data(UNCOLLECTED_RATE))]
    );
    assert!(
        type_in(
            &with_uncollected_rate("MOD(1, AGG(collection_rate))"),
            CATALOG,
            OssieFormat::Release
        )
        .1
        .is_ok()
    );
}

#[test]
fn der_2_4_der_5_1_4_e0c_34_a_cycle_is_reported_once_at_its_first_metric() {
    let model = conformance_model("e0-derived-cycle-001");
    assert_eq!(rejected(model), [("E0C-34", data(COLLECTION_RATE))]);
    let errors = type_in(model, CATALOG, OssieFormat::Release)
        .1
        .expect_err("the model is rejected");
    assert!(
        errors[0]
            .message
            .ends_with("collection_rate → uncollected_rate → collection_rate"),
        "{}",
        errors[0].message
    );
}

#[test]
fn der_5_1_4_a_metric_that_reads_itself() {
    let errors = type_in(
        &with_uncollected_rate("1 - AGG(uncollected_rate)"),
        CATALOG,
        OssieFormat::Release,
    )
    .1
    .expect_err("the model is rejected");
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0]
            .message
            .ends_with("uncollected_rate → uncollected_rate")
    );
}

#[test]
fn der_5_1_4_each_cycle_starts_at_its_first_metric_in_document_order() {
    let model = with(
        "AGG(billed) / AGG(customer_count)",
        "AGG(uncollected_rate) / AGG(customer_count)",
    );
    let model = model.replacen(
        "AGG(collected) / AGG(billed)",
        "AGG(billed_per_customer)",
        1,
    );
    let errors = type_in(&model, CATALOG, OssieFormat::Release)
        .1
        .expect_err("the model is rejected");
    let messages: Vec<&str> = errors
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect();
    assert_eq!(
        messages,
        [
            "the derived metrics depend on each other: collection_rate → billed_per_customer → uncollected_rate → collection_rate"
        ]
    );
    assert_eq!(errors[0].pointer.to_string(), data(COLLECTION_RATE));
}

#[test]
fn der_5_1_5_a_formula_that_reads_an_unusable_metric_gets_no_type() {
    let (_, derived) = type_in(
        conformance_model("e0-derived-unusable"),
        CATALOG,
        OssieFormat::Release,
    );
    let derived = derived.expect("the formulas are accepted");
    assert!(derived.elements.is_empty() && derived.warnings.is_empty());
}

#[test]
fn der_5_1_2_e0f_02_agg_in_a_field_or_a_dataset_metric() {
    let (typing, _) = type_in(
        conformance_model("e0-derived-unusable"),
        CATALOG,
        OssieFormat::Release,
    );
    assert_eq!(
        summary(&typing.warnings),
        [(
            "E0F-02",
            "/semantic_model/0/datasets/0/fields/3/expression".to_owned()
        )]
    );
    let model = conformance_model("e0-derived-development").replacen(
        "{sql: SUM(amount)}",
        "{sql: AGG(order_count)}",
        1,
    );
    let (typing, derived) = type_in(&model, DEVELOPMENT_CATALOG, OssieFormat::Development);
    assert_eq!(
        summary(&typing.warnings),
        [("E0F-02", "/datasets/0/metrics/0/expression".to_owned())]
    );
    assert!(
        derived
            .expect("the formulas are accepted")
            .elements
            .is_empty()
    );
}

#[test]
fn der_2_2_a_development_metric_of_dataset_metrics_is_typed() {
    let (_, derived) = type_in(
        conformance_model("e0-derived-development"),
        DEVELOPMENT_CATALOG,
        OssieFormat::Development,
    );
    let derived = derived.expect("the formulas are accepted");
    assert_eq!(type_of(&derived, "/metrics/0"), Some(double()));
}

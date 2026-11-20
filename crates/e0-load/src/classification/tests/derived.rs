//! The classification of derived metrics: DER-5.1.5, DER-5.1.6 and E0G-5.2 to E0G-5.4.

use std::collections::BTreeSet;

use menura_contracts::types::audit_record::OssieFormat;

use super::super::Classification;
use super::super::output::{Additivity, QueryError};
use super::{ClassificationCode, ClassifiedMetric, Probe, classify_in_format, probe, summary};
use crate::extensions::ExtensionCode;
use crate::extensions::derived::fixture::{
    BILLED, BILLED_PER_CUSTOMER, CATALOG, COLLECTED, COLLECTION_RATE, CUSTOMER_COUNT,
    DEVELOPMENT_CATALOG, MODEL, UNCOLLECTED_RATE, conformance_model, with,
};
use crate::names::Name;
use crate::pointer::JsonPointer;
use menura_contracts::types::common::{FloatBits, LogicalType};
use menura_expressions::typing::TypingCode;

fn classify(model: &str) -> Classification {
    classify_in_format(model, CATALOG, OssieFormat::Release)
}

fn classified(classification: &Classification, pointer: &str) -> ClassifiedMetric {
    match probe(classification, pointer) {
        Probe::Classified(metric) => *metric,
        other => panic!("{pointer} must be classified, found {other:?}"),
    }
}

fn home_datasets(names: &[&str]) -> BTreeSet<Name> {
    names.iter().map(|name| Name::regular(name)).collect()
}

fn pointers(metric: &ClassifiedMetric) -> Vec<&str> {
    metric.inputs.iter().map(JsonPointer::as_str).collect()
}

/// [`MODEL`] with the expression of the base metric `name`, `SUM(invoices.<name>)`, replaced
/// by `expression`.
fn with_base(name: &str, expression: &str) -> String {
    with(
        &format!("expression: 'SUM(invoices.{name})'"),
        &format!("expression: '{}'", expression.replace('\'', "''")),
    )
}

#[test]
fn der_5_1_6_the_example_is_non_additive_and_follows_its_inputs() {
    let classification = classify(MODEL);
    assert!(classification.warnings.is_empty());
    let rate = classified(&classification, COLLECTION_RATE);
    assert_eq!(rate.additivity, Additivity::NonAdditive);
    assert_eq!(pointers(&rate), [COLLECTED, BILLED]);
    assert!(rate.components.is_empty() && rate.declaration.is_none());
    assert_eq!(rate.home_datasets, home_datasets(&["invoices"]));
    assert!(rate.recomposable);
    assert_eq!(
        rate.result_type,
        LogicalType::Float {
            bits: FloatBits::SixtyFour
        }
    );
    let billed = classified(&classification, BILLED);
    assert_eq!(billed.additivity, Additivity::Additive);
    assert_eq!(billed.home_datasets, home_datasets(&["invoices"]));
    assert!(billed.inputs.is_empty());
}

#[test]
fn e0g_5_2_e0g_5_3_home_datasets_and_recomposability_follow_every_input() {
    let classification = classify(MODEL);
    let uncollected = classified(&classification, UNCOLLECTED_RATE);
    assert_eq!(pointers(&uncollected), [COLLECTION_RATE]);
    assert_eq!(uncollected.home_datasets, home_datasets(&["invoices"]));
    assert!(uncollected.recomposable);
    let per_customer = classified(&classification, BILLED_PER_CUSTOMER);
    assert_eq!(pointers(&per_customer), [BILLED, CUSTOMER_COUNT]);
    assert_eq!(
        per_customer.home_datasets,
        home_datasets(&["invoices", "customers"])
    );
    assert!(!per_customer.recomposable);
    assert_eq!(per_customer.additivity, Additivity::NonAdditive);
}

#[test]
fn der_5_1_6_e0g_2_does_not_apply_to_a_derived_metric() {
    let classification = classify(MODEL);
    for metric in [COLLECTION_RATE, UNCOLLECTED_RATE, BILLED_PER_CUSTOMER] {
        assert!(matches!(
            probe(&classification, metric),
            Probe::Classified(_)
        ));
    }
}

#[test]
fn der_5_1_5_an_unusable_input_makes_the_derived_metric_unusable() {
    let classification = classify(conformance_model("e0-derived-unusable"));
    let twice_share = "/semantic_model/0/metrics/2";
    assert_eq!(
        probe(&classification, twice_share),
        Probe::UnusableInput(QueryError::Extension(
            ExtensionCode::AggregateOutsideDerived
        ))
    );
    assert_eq!(
        summary(&classification),
        [("E0C-37", twice_share.to_owned())]
    );
    assert_eq!(
        probe(&classification, "/semantic_model/0/metrics/1"),
        Probe::NotClassified,
        "E0C-37 is reported by E0.d"
    );
}

#[test]
fn der_5_1_5_the_query_error_is_that_of_the_first_unusable_input_in_written_order() {
    let model = with_base("collected", "SUM(invoices.collected) + invoices.collected");
    let model = model.replacen(
        "expression: 'SUM(invoices.billed)'",
        "expression: '1 + 2'",
        1,
    );
    let classification = classify(&model);
    let unaggregated = Probe::Unusable(ClassificationCode::UnaggregatedFinerGrainReference);
    assert_eq!(probe(&classification, COLLECTION_RATE), unaggregated);
    assert_eq!(probe(&classification, UNCOLLECTED_RATE), unaggregated);
    assert_eq!(
        probe(&classification, BILLED_PER_CUSTOMER),
        Probe::Unusable(ClassificationCode::NoAggregate)
    );
    assert_eq!(
        summary(&classification),
        [
            ("E0G-02", BILLED.to_owned()),
            ("E_UNAGGREGATED_FINER_GRAIN_REFERENCE", COLLECTED.to_owned()),
            (
                "E_UNAGGREGATED_FINER_GRAIN_REFERENCE",
                COLLECTION_RATE.to_owned()
            ),
            (
                "E_UNAGGREGATED_FINER_GRAIN_REFERENCE",
                UNCOLLECTED_RATE.to_owned()
            ),
            ("E0G-02", BILLED_PER_CUSTOMER.to_owned()),
        ]
    );
}

#[test]
fn der_5_1_5_an_input_unusable_after_e0f_gives_its_code() {
    let classification = classify(&with_base("billed", "SUM(invoices.billed) + 'x'"));
    assert_eq!(
        probe(&classification, COLLECTION_RATE),
        Probe::UnusableInput(QueryError::Typing(TypingCode::IncompatibleTypes))
    );
    assert_eq!(probe(&classification, BILLED), Probe::NotClassified);
}

#[test]
fn der_2_2_e0g_5_2_a_development_metric_of_dataset_metrics() {
    let classification = classify_in_format(
        conformance_model("e0-derived-development"),
        DEVELOPMENT_CATALOG,
        OssieFormat::Development,
    );
    let average = classified(&classification, "/metrics/0");
    assert_eq!(
        pointers(&average),
        ["/datasets/0/metrics/0", "/datasets/0/metrics/1"]
    );
    assert_eq!(average.home_datasets, home_datasets(&["orders"]));
    assert!(average.recomposable);
    assert_eq!(average.additivity, Additivity::NonAdditive);
}

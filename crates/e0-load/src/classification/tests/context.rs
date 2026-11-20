//! The classification of context metrics: CTX-3.6, CTX-3.7, CTX-5.1.6, CTX-5.1.7 and
//! `W0G-01`.

use std::collections::BTreeSet;

use menura_contracts::types::audit_record::OssieFormat;
use menura_contracts::types::common::{FloatBits, IntegerBits, LogicalType};

use super::super::Classification;
use super::super::output::{Additivity, QueryError};
use super::{ClassifiedMetric, Probe, classify_in_format, probe, summary};
use crate::extensions::ExtensionCode;
use crate::extensions::context::fixture::{
    BILLED, BILLED_CHANGE, BILLED_PRIOR_YEAR, BILLED_SHARE, BILLED_YTD, CATALOG, MODEL,
    conformance_model, with,
};
use crate::names::Name;
use crate::pointer::JsonPointer;

fn classify(model: &str) -> Classification {
    classify_in_format(model, CATALOG, OssieFormat::Release)
}

fn classified(classification: &Classification, pointer: &str) -> ClassifiedMetric {
    match probe(classification, pointer) {
        Probe::Classified(metric) => *metric,
        other => panic!("{pointer} must be classified, found {other:?}"),
    }
}

fn invoices() -> BTreeSet<Name> {
    BTreeSet::from([Name::regular("invoices")])
}

fn inputs(metric: &ClassifiedMetric) -> Vec<&str> {
    metric.inputs.iter().map(JsonPointer::as_str).collect()
}

#[test]
fn ctx_5_1_7_a_context_metric_is_classified_from_its_base() {
    let classification = classify(MODEL);
    assert!(classification.warnings.is_empty() && classification.unusable.is_empty());
    for metric in [BILLED_YTD, BILLED_PRIOR_YEAR, BILLED_SHARE] {
        let context = classified(&classification, metric);
        assert!(
            context.components.is_empty(),
            "{metric} is not a base metric"
        );
        assert_eq!(inputs(&context), [BILLED], "{metric}");
        assert_eq!(context.home_datasets, invoices(), "{metric}");
        assert_eq!(context.additivity, Additivity::NonAdditive, "{metric}");
        assert!(context.declaration.is_none(), "{metric}");
        assert!(context.recomposable, "SUM is recomposable");
        assert!(context.nullable, "{metric}");
    }
    let sum_of_integer = LogicalType::Integer {
        bits: IntegerBits::OneHundredTwentyEight,
        signed: true,
    };
    assert_eq!(
        classified(&classification, BILLED_YTD).result_type,
        sum_of_integer
    );
    assert_eq!(
        classified(&classification, BILLED_SHARE).result_type,
        LogicalType::Float {
            bits: FloatBits::SixtyFour
        }
    );
}

#[test]
fn ctx_3_6_a_context_metric_is_recomposable_only_if_its_base_is() {
    let model = with(
        "expression: {dialects: [{dialect: ANSI_SQL, expression: 'SUM(invoices.billed)'}]}",
        "expression: {dialects: [{dialect: ANSI_SQL, expression: 'COUNT(DISTINCT invoices.billed)'}]}",
    );
    let classification = classify(&model);
    assert!(!classified(&classification, BILLED).recomposable);
    for metric in [BILLED_YTD, BILLED_PRIOR_YEAR, BILLED_SHARE] {
        assert!(
            !classified(&classification, metric).recomposable,
            "{metric}"
        );
    }
}

#[test]
fn ctx_3_7_a_derived_metric_reads_a_context_metric() {
    let classification = classify(MODEL);
    let change = classified(&classification, BILLED_CHANGE);
    assert_eq!(inputs(&change), [BILLED, BILLED_PRIOR_YEAR]);
    assert_eq!(change.home_datasets, invoices());
}

#[test]
fn ctx_5_1_6_w0g_01_a_context_metric_whose_base_is_unusable() {
    let classification = classify(conformance_model("e0-context-unusable"));
    assert_eq!(
        probe(&classification, BILLED_SHARE),
        Probe::UnusableInput(QueryError::Extension(
            ExtensionCode::AggregateOutsideDerived
        ))
    );
    assert_eq!(
        summary(&classification),
        [("E0C-37", BILLED_SHARE.to_owned())],
        "one W0G-01 at the context metric, with the query error of its base"
    );
}

#[test]
fn ctx_5_1_7_the_development_format_classifies_a_context_metric_over_a_derived_base() {
    let classification = classify_in_format(
        conformance_model("e0-context-development"),
        CATALOG,
        OssieFormat::Development,
    );
    for metric in ["/metrics/1", "/metrics/2"] {
        let context = classified(&classification, metric);
        assert_eq!(inputs(&context), ["/metrics/0"], "{metric}");
        assert_eq!(context.home_datasets, invoices(), "{metric}");
    }
}

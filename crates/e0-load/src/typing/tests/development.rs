//! The development format (section 4.5, OSS-5.1): the `sql` expressions and the dataset-scoped
//! metrics are typed, reading the references of a dataset metric as E0.d resolves them.

use menura_contracts::types::audit_record::OssieFormat;
use menura_contracts::types::common::LogicalType;

use super::*;
use crate::typing::value_type::{decimal, huge_integer};

const MODEL: &str =
    include_str!("../../../../../conformance/models/e0-typing-development/model.yaml");
const CATALOG: &str =
    include_str!("../../../../../conformance/models/e0-typing-development/catalog-schema.json");

fn type_development(model: &str) -> Typing {
    type_in_format(model, CATALOG, OssieFormat::Development)
}

fn typed(typing: &Typing, kind: ElementKind, pointer: &str) -> Option<Typed> {
    typing
        .elements
        .iter()
        .find(|element| element.kind == kind && element.pointer.as_str() == pointer)
        .map(|element| element.typed.clone())
}

#[test]
fn oss_5_1_sql_expressions_and_dataset_metrics_are_typed() {
    let typing = type_development(MODEL);
    assert_eq!(
        summary(&typing.warnings),
        [
            pair("E0F-04", "/datasets/0/fields/4/expression"),
            pair("E0F-03", "/datasets/0/fields/5/expression"),
            pair("E0F-04", "/datasets/0/metrics/2/expression"),
            pair("E0F-07", "/datasets/0/metrics/3/expression"),
            pair("E0F-03", "/datasets/0/metrics/4/expression"),
            pair("E0F-03", "/metrics/1/expression"),
        ]
    );
    let dataset_metric = |index| {
        typed(
            &typing,
            ElementKind::DatasetMetric,
            &format!("/datasets/0/metrics/{index}"),
        )
    };
    assert_eq!(
        dataset_metric(0),
        Some(Typed::value(huge_integer(), true)),
        "a bare name is a field of the dataset"
    );
    assert_eq!(
        dataset_metric(1),
        Some(Typed::value(decimal(38, 2), true)),
        "`source.column` is a column of the catalog relation"
    );
    assert_eq!(
        typed(&typing, ElementKind::Metric, "/metrics/0"),
        Some(Typed::value(decimal(38, 2), true))
    );
    assert_eq!(
        typed(&typing, ElementKind::Field, "/datasets/0/fields/1"),
        Some(Typed::value(LogicalType::Text, false))
    );
}

#[test]
fn oss_5_1_dataset_metric_kinds_of_unusable_elements() {
    let typing = type_development(MODEL);
    let unusable: Vec<(ElementKind, String)> = typing
        .unusable
        .iter()
        .map(|element| (element.kind, element.pointer.to_string()))
        .collect();
    assert_eq!(
        unusable,
        [
            (ElementKind::Field, "/datasets/0/fields/4".to_owned()),
            (ElementKind::Field, "/datasets/0/fields/5".to_owned()),
            (
                ElementKind::DatasetMetric,
                "/datasets/0/metrics/2".to_owned()
            ),
            (
                ElementKind::DatasetMetric,
                "/datasets/0/metrics/3".to_owned()
            ),
            (
                ElementKind::DatasetMetric,
                "/datasets/0/metrics/4".to_owned()
            ),
            (ElementKind::Metric, "/metrics/1".to_owned()),
        ]
    );
}

#[test]
fn oss_5_1_dataset_metric_references_reported_by_e0d_get_no_type() {
    for expression in [
        "SUM(tax)",
        "SUM(source.tax)",
        "SUM(orders.quantity)",
        "SUM(payload)",
    ] {
        let model = MODEL.replace(
            "expression: {sql: SUM(source.price)}",
            &format!("expression: {{sql: '{expression}'}}"),
        );
        let typing = type_development(&model);
        assert_eq!(
            typed(&typing, ElementKind::DatasetMetric, "/datasets/0/metrics/1"),
            None,
            "{expression}"
        );
        assert!(
            !typing
                .unusable
                .iter()
                .any(|element| element.pointer.as_str() == "/datasets/0/metrics/1"),
            "{expression}"
        );
    }
    let model = MODEL.replace(
        "expression: {sql: SUM(source.price)}",
        "expression: {sql: SUM(SOURCE.price)}",
    );
    assert_eq!(
        typed(
            &type_development(&model),
            ElementKind::DatasetMetric,
            "/datasets/0/metrics/1"
        ),
        Some(Typed::value(decimal(38, 2), true)),
        "the qualifier `source` is a regular identifier"
    );
}

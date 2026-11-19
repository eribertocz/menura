//! The summary extension in E0.c: SUM-2, SUM-6, SUM-6.1.1, SUM-6.1.6, `E0C-02`, `E0C-03`,
//! `E0C-50` (a missing `of`), `E0C-55` and `E0C-56`, and which datasets are summaries.

use semantic_compiler_contracts::types::audit_record::OssieFormat;

use super::datasets::summary_datasets;
use super::fixture::{
    ACCESS, AGGREGATE, MODEL, SUMMARY, SUMMARY_DATA, conformance_model, with_aggregate,
    with_changed, with_members,
};
use super::model::Grain;
use super::{SummaryDimension, SummaryMeasure};
use crate::extensions::output::{Extensions, ExtensionsFailure};
use crate::extensions::{DEFAULT_NAMESPACE, validate_extensions};
use crate::pointer::JsonPointer;
use crate::read::read_document;
use crate::structure::validate_structure;

fn validate_in(model: &str, format: OssieFormat) -> Result<Extensions, ExtensionsFailure> {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure = validate_structure(&document, format, None).expect("the structure is valid");
    validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
}

/// The code and JSON Pointer of every rejecting diagnostic of `model`, read in `format`.
fn rejected_in(model: &str, format: OssieFormat) -> Vec<(&'static str, String)> {
    validate_in(model, format)
        .expect_err("the extensions are rejected")
        .errors
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect()
}

/// The codes that reject `model`, each at the `data` member of the summary entry.
fn codes(model: &str) -> Vec<&'static str> {
    codes_in(model, OssieFormat::Release, SUMMARY_DATA)
}

fn codes_in(model: &str, format: OssieFormat, data: &str) -> Vec<&'static str> {
    rejected_in(model, format)
        .into_iter()
        .map(|(code, pointer)| {
            assert_eq!(pointer, data, "{code}");
            code
        })
        .collect()
}

/// The codes that reject the summary whose `aggregate` is `aggregate`.
fn aggregate_codes(aggregate: &str) -> Vec<&'static str> {
    codes(&with_aggregate(aggregate))
}

fn changed_codes(from: &str, to: &str) -> Vec<&'static str> {
    codes(&with_changed(from, to))
}

#[test]
fn sum_2_sum_6_1_1_the_valid_summary_is_read() {
    let extensions = validate_in(MODEL, OssieFormat::Release).expect("the extensions are accepted");
    assert!(extensions.warnings.is_empty());
    assert_eq!(extensions.summaries.len(), 1);
    let summary = &extensions.summaries[0];
    assert_eq!(summary.dataset.to_string(), SUMMARY);
    assert_eq!(summary.data.to_string(), SUMMARY_DATA);
    assert_eq!(summary.of, "orders");
    assert_eq!(
        summary.dimensions,
        [
            SummaryDimension {
                field: "order_date".to_owned(),
                base: "order_date".to_owned(),
                grain: Some(Grain::Day),
            },
            SummaryDimension {
                field: "region".to_owned(),
                base: "region".to_owned(),
                grain: None,
            },
        ]
    );
    assert_eq!(
        summary.measures,
        [
            SummaryMeasure {
                field: "total_amount".to_owned(),
                base_aggregate: "SUM(orders.amount)".to_owned(),
            },
            SummaryMeasure {
                field: "order_count".to_owned(),
                base_aggregate: "COUNT(orders.order_id)".to_owned(),
            },
        ]
    );
    assert_eq!(summary.coverage_field, "order_date");
    assert!(extensions.is_summary(&JsonPointer::from_text(SUMMARY)));
}

#[test]
fn sum_6_1_1_every_grain_is_read() {
    for (written, grain) in [
        ("day", Grain::Day),
        ("week", Grain::Week),
        ("month", Grain::Month),
        ("quarter", Grain::Quarter),
        ("year", Grain::Year),
    ] {
        let model = with_changed(r#""grain": "day""#, &format!(r#""grain": "{written}""#));
        let extensions =
            validate_in(&model, OssieFormat::Release).expect("the extensions are accepted");
        assert_eq!(extensions.summaries[0].dimensions[0].grain, Some(grain));
    }
}

#[test]
fn sum_6_1_1_e0c_50_a_missing_of() {
    assert_eq!(
        codes(conformance_model("e0-summary-missing-of-001")),
        ["E0C-50"]
    );
}

#[test]
fn sum_6_1_1_e0c_50_with_other_problems() {
    let aggregate = AGGREGATE.replacen(r#""of": "orders", "#, "", 1);
    let aggregate = aggregate.replacen(
        r#""coverage_field": "order_date""#,
        r#""coverage_field": 1"#,
        1,
    );
    assert_eq!(aggregate_codes(&aggregate), ["E0C-03", "E0C-50"]);
}

#[test]
fn sum_6_1_1_e0c_03_the_shape_of_aggregate() {
    assert_eq!(aggregate_codes("5"), ["E0C-03"]);
    assert_eq!(aggregate_codes("[]"), ["E0C-03"]);
    assert_eq!(changed_codes(r#""of": "orders""#, r#""of": 1"#), ["E0C-03"]);
    assert_eq!(
        changed_codes(
            r#""coverage_field": "order_date""#,
            r#""coverage_field": null"#
        ),
        ["E0C-03"]
    );
    let without_coverage = AGGREGATE.replacen(r#", "coverage_field": "order_date""#, "", 1);
    assert_eq!(aggregate_codes(&without_coverage), ["E0C-03"]);
}

#[test]
fn sum_6_1_1_e0c_03_the_shape_of_dimensions() {
    let dimensions = r#"[{"field": "order_date", "base": "order_date", "grain": "day"}, {"field": "region", "base": "region"}]"#;
    for wrong in [
        "[]",
        r#""order_date""#,
        r#"["order_date"]"#,
        r#"[{"base": "order_date", "grain": "day"}]"#,
        r#"[{"field": "order_date", "grain": "day"}]"#,
        r#"[{"field": 1, "base": "order_date"}]"#,
        r#"[{"field": "order_date", "base": "order_date", "grain": "hour"}]"#,
        r#"[{"field": "order_date", "base": "order_date", "grain": "Day"}]"#,
        r#"[{"field": "order_date", "base": "order_date", "grain": null}]"#,
    ] {
        assert_eq!(changed_codes(dimensions, wrong), ["E0C-03"], "{wrong}");
    }
    let without = AGGREGATE.replacen(&format!(r#""dimensions": {dimensions}, "#), "", 1);
    assert_eq!(aggregate_codes(&without), ["E0C-03"]);
}

#[test]
fn sum_6_1_1_e0c_03_the_shape_of_measures() {
    let measures = r#"[{"field": "total_amount", "base_aggregate": "SUM(orders.amount)"}, {"field": "order_count", "base_aggregate": "COUNT(orders.order_id)"}]"#;
    for wrong in [
        "[]",
        "{}",
        r#"[{"field": "total_amount"}]"#,
        r#"[{"base_aggregate": "SUM(orders.amount)"}]"#,
        r#"[{"field": "total_amount", "base_aggregate": 1}]"#,
    ] {
        assert_eq!(changed_codes(measures, wrong), ["E0C-03"], "{wrong}");
    }
    let without = AGGREGATE.replacen(&format!(r#""measures": {measures}, "#), "", 1);
    assert_eq!(aggregate_codes(&without), ["E0C-03"]);
}

#[test]
fn sum_6_1_1_e0c_02_unknown_keys() {
    assert_eq!(
        codes(conformance_model("e0-summary-unknown-key-001")),
        ["E0C-02"]
    );
    assert_eq!(
        changed_codes(r#""base": "region"}"#, r#""base": "region", "role": "x"}"#),
        ["E0C-02"]
    );
    assert_eq!(
        changed_codes(
            r#""base_aggregate": "SUM(orders.amount)"}"#,
            r#""base_aggregate": "SUM(orders.amount)", "grain": "day"}"#
        ),
        ["E0C-02"]
    );
    assert_eq!(
        changed_codes(r#""of": "orders""#, r#""of": "orders", "owner": 1"#),
        ["E0C-02"]
    );
}

#[test]
fn sum_6_1_6_e0c_55_a_summary_with_row_access() {
    assert_eq!(
        codes(conformance_model("e0-summary-row-access-001")),
        ["E0C-55"]
    );
}

#[test]
fn sum_6_1_6_e0c_55_a_summary_with_dataset_metrics() {
    let data = "/datasets/1/nonportable_extensions/0/data";
    assert_eq!(
        codes_in(
            conformance_model("e0-summary-dataset-metrics-001"),
            OssieFormat::Development,
            data
        ),
        ["E0C-55"]
    );
}

#[test]
fn sum_6_1_6_the_development_format() {
    let extensions = validate_in(
        conformance_model("e0-summaries-development"),
        OssieFormat::Development,
    )
    .expect("the extensions are accepted");
    assert_eq!(extensions.summaries.len(), 1);
    assert_eq!(extensions.summaries[0].dataset.to_string(), "/datasets/1");
}

#[test]
fn sum_6_1_6_e0c_56_a_summary_without_access() {
    assert_eq!(
        codes(conformance_model("e0-summary-access-001")),
        ["E0C-56"]
    );
}

#[test]
fn sum_6_1_6_e0c_56_a_valid_mode_other_than_internal() {
    let aggregate = format!(r#""aggregate": {AGGREGATE}"#);
    let authenticated = format!(r#""access": {{"mode": "authenticated"}}, {aggregate}"#);
    assert_eq!(codes(&with_members(&authenticated)), ["E0C-56"]);
    let roles = format!(r#""access": {{"mode": "roles", "roles": ["analyst"]}}, {aggregate}"#);
    assert_eq!(codes(&with_members(&roles)), ["E0C-56"]);
}

#[test]
fn sum_6_1_6_a_missing_or_invalid_mode_is_only_e0c_41() {
    let aggregate = format!(r#""aggregate": {AGGREGATE}"#);
    let without_mode = format!(r#""access": {{"roles": ["analyst"]}}, {aggregate}"#);
    assert_eq!(codes(&with_members(&without_mode)), ["E0C-41"]);
    let public = format!(r#""access": {{"mode": "public"}}, {aggregate}"#);
    assert_eq!(codes(&with_members(&public)), ["E0C-41"]);
    let internal_roles =
        format!(r#""access": {{"mode": "internal", "roles": ["analyst"]}}, {aggregate}"#);
    assert_eq!(codes(&with_members(&internal_roles)), ["E0C-43"]);
}

/// The summary datasets of `model`, read in `format`, as JSON Pointer texts.
fn summaries_in(model: &str, format: OssieFormat) -> Vec<String> {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure = validate_structure(&document, format, None).expect("the structure is valid");
    summary_datasets(&structure.semantic_model, DEFAULT_NAMESPACE)
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn summaries(model: &str) -> Vec<String> {
    summaries_in(model, OssieFormat::Release)
}

#[test]
fn sum_6_1_1_a_dataset_is_a_summary_by_the_aggregate_key() {
    assert_eq!(summaries(MODEL), [SUMMARY]);
    assert_eq!(
        summaries_in(
            conformance_model("e0-summaries-development"),
            OssieFormat::Development
        ),
        ["/datasets/1"]
    );
    let without = with_members(ACCESS);
    assert!(summaries(&without).is_empty());
}

#[test]
fn sum_6_1_1_sum_6_1_7_a_summary_whatever_e0c_reports() {
    assert_eq!(
        summaries(conformance_model("e0-summary-missing-of-001")),
        [SUMMARY]
    );
    assert_eq!(summaries(&with_aggregate("5")), [SUMMARY]);
    let unknown = format!(r#"{ACCESS}, "owner": 1, "aggregate": {{}}"#);
    assert_eq!(summaries(&with_members(&unknown)), [SUMMARY]);
    let other_version = MODEL.replacen(
        r#""version": 1, "access": {"mode": "internal"}"#,
        r#""version": 2, "access": {"mode": "internal"}"#,
        1,
    );
    assert_ne!(other_version, MODEL);
    assert_eq!(summaries(&other_version), [SUMMARY]);
}

#[test]
fn sum_6_1_1_an_aggregate_key_of_someone_else_is_not_a_summary() {
    let foreign = MODEL.replacen(
        r#""namespace": "semantic_compiler", "version": 1, "access": {"mode": "internal"}"#,
        r#""namespace": "other", "version": 1, "access": {"mode": "internal"}"#,
        1,
    );
    assert_ne!(foreign, MODEL);
    assert!(summaries(&foreign).is_empty());
}

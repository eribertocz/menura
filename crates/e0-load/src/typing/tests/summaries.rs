//! The summary extension in E0.f: SUM-2.1, SUM-6.1.3 to SUM-6.1.5, SUM-6.1.7, `E0C-52`,
//! `E0C-53` and `E0C-57`.

use menura_contracts::admit_catalog_schema;
use menura_contracts::types::audit_record::OssieFormat;

use super::{ElementKind, Typing};
use crate::extensions::summaries::datasets::summary_datasets;
use crate::extensions::summaries::fixture::{
    CATALOG, KIND_CATALOG, MODEL, SUMMARY_DATA, conformance_model, with_changed, with_text,
};
use crate::extensions::{DEFAULT_NAMESPACE, validate_extensions};
use crate::read::read_document;
use crate::references::derived::DerivedMetrics;
use crate::structure::validate_structure;
use crate::typing::summaries::type_summaries;
use crate::typing::type_expressions;

/// The typing of `model`, read in `format`, against `catalog`, with the summaries of its raw
/// entries (SUM-6.1.7).
fn typing_of(model: &str, catalog: &str, format: OssieFormat) -> Typing {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure = validate_structure(&document, format, None).expect("the structure is valid");
    let catalog = admit_catalog_schema(catalog).expect("the catalog schema is admitted");
    let summaries = summary_datasets(&structure.semantic_model, DEFAULT_NAMESPACE);
    type_expressions(
        &document,
        &structure,
        &catalog,
        &DerivedMetrics::default(),
        &summaries,
    )
}

/// The codes of every diagnostic of the summaries of `model`, each at the `data` member
/// `data`.
fn codes_in(model: &str, catalog: &str, format: OssieFormat, data: &str) -> Vec<&'static str> {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure = validate_structure(&document, format, None).expect("the structure is valid");
    let extensions = validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
        .expect("the extensions are accepted");
    let typing = typing_of(model, catalog, format);
    match type_summaries(&structure, &typing, &extensions) {
        Ok(()) => Vec::new(),
        Err(errors) => errors
            .iter()
            .map(|diagnostic| {
                assert_eq!(diagnostic.pointer.to_string(), data, "{}", diagnostic.code);
                diagnostic.code.as_str()
            })
            .collect(),
    }
}

fn codes(model: &str) -> Vec<&'static str> {
    codes_in(model, CATALOG, OssieFormat::Release, SUMMARY_DATA)
}

/// The codes of the summary whose first measure has the `base_aggregate` `text`.
fn aggregate_codes(text: &str) -> Vec<&'static str> {
    codes(&with_changed("SUM(orders.amount)", text))
}

#[test]
fn sum_6_1_3_the_valid_summaries_type() {
    assert!(codes(MODEL).is_empty());
    assert!(
        codes_in(
            conformance_model("e0-summaries-development"),
            CATALOG,
            OssieFormat::Development,
            "/datasets/1/nonportable_extensions/0/data"
        )
        .is_empty()
    );
}

#[test]
fn sum_2_1_sum_6_1_3_the_accepted_base_aggregates() {
    for text in [
        "SUM(orders.amount)",
        "sum(orders.amount)",
        "MIN(orders.amount)",
        "MAX(orders.amount)",
        "SUM(orders.amount) FILTER (WHERE orders.region = 'NORTH')",
        "SUM(CASE WHEN orders.region = 'NORTH' THEN orders.amount ELSE 0 END)",
        "SUM(orders.amount * 2)",
    ] {
        assert!(aggregate_codes(text).is_empty(), "{text}");
    }
    assert!(
        codes(&with_changed(
            "COUNT(orders.order_id)",
            "COUNT(orders.region) FILTER (WHERE orders.amount > 0)"
        ))
        .is_empty()
    );
}

#[test]
fn sum_2_1_sum_6_1_3_e0c_52_an_invalid_base_aggregate() {
    assert_eq!(
        codes(conformance_model("e0-summary-base-aggregate-001")),
        ["E0C-52"]
    );
    for text in [
        "SUM(orders.amount",
        "orders.amount",
        "COUNT(*)",
        "COUNT(DISTINCT orders.amount)",
        "SUM(DISTINCT orders.amount)",
        "STDDEV(orders.amount)",
        "SUM(orders.amount) + 1",
        "SUM(orders.amount) + SUM(orders.order_id)",
        "SUM(amount)",
        "SUM(orders_daily.total_amount)",
        "SUM(orders.amount) FILTER (WHERE orders_daily.region = 'NORTH')",
        "AGG(total_amount)",
        "SUM(CASE WHEN USER_ATTRIBUTE('limit') > 0 THEN orders.amount ELSE 0 END)",
        "SUM(SUM(orders.amount))",
    ] {
        assert_eq!(aggregate_codes(text), ["E0C-52"], "{text}");
    }
}

#[test]
fn sum_6_1_3_e0c_52_a_base_aggregate_that_does_not_type() {
    assert_eq!(aggregate_codes("SUM(orders.region)"), ["E0C-52"]);
    assert_eq!(
        aggregate_codes("SUM(orders.amount) FILTER (WHERE orders.amount)"),
        ["E0C-52"]
    );
}

#[test]
fn sum_6_1_3_types_are_checked_only_on_typed_fields() {
    let untyped = with_text("expression: 'amount'}", "expression: 'UPPER(amount)'}");
    assert!(codes(&untyped).is_empty());
    let untyped_text = untyped.replacen("SUM(orders.amount)", "SUM(orders.region) + 1", 1);
    assert_eq!(codes(&untyped_text), ["E0C-52"]);
}

#[test]
fn sum_6_1_4_e0c_53_a_time_dimension_without_grain() {
    assert_eq!(codes(conformance_model("e0-summary-grain-001")), ["E0C-53"]);
}

#[test]
fn sum_6_1_4_e0c_53_a_grain_on_a_field_that_is_not_a_time_field() {
    assert_eq!(
        codes(&with_changed(
            r#"{"field": "region", "base": "region"}"#,
            r#"{"field": "region", "base": "region", "grain": "month"}"#
        )),
        ["E0C-53"]
    );
}

#[test]
fn sum_6_1_4_the_grain_is_checked_only_on_a_typed_base() {
    let untyped = with_text("expression: 'region'}", "expression: 'UPPER(order_date)'}");
    let model = untyped.replacen(
        r#"{"field": "region", "base": "region"}"#,
        r#"{"field": "region", "base": "region", "grain": "month"}"#,
        1,
    );
    assert!(!codes(&model).contains(&"E0C-53"));
}

#[test]
fn sum_6_1_5_e0c_57_a_measure_of_another_kind() {
    assert_eq!(
        codes_in(
            conformance_model("e0-summary-kind-001"),
            KIND_CATALOG,
            OssieFormat::Release,
            SUMMARY_DATA
        ),
        ["E0C-57"]
    );
}

#[test]
fn sum_6_1_5_e0c_57_a_dimension_of_another_kind() {
    let model = with_changed(
        r#"{"field": "region", "base": "region"}"#,
        r#"{"field": "region", "base": "amount"}"#,
    );
    assert_eq!(codes(&model), ["E0C-57"]);
}

#[test]
fn sum_6_1_5_kinds_compare_under_e0f_4_1() {
    assert!(aggregate_codes("MAX(orders.order_id)").is_empty());
    assert!(
        codes(&with_changed(
            "COUNT(orders.order_id)",
            "SUM(orders.amount)"
        ))
        .is_empty()
    );
}

#[test]
fn sum_6_1_3_sum_6_1_5_a_measure_with_e0c_52_is_not_checked_for_its_kind() {
    let model = conformance_model("e0-summary-kind-001").replacen(
        "SUM(orders.amount)",
        "AVG(orders.amount)",
        1,
    );
    assert_eq!(
        codes_in(&model, KIND_CATALOG, OssieFormat::Release, SUMMARY_DATA),
        ["E0C-52"]
    );
}

#[test]
fn sum_6_1_4_sum_6_1_5_several_codes_of_one_summary() {
    let model = with_changed(
        r#"{"field": "region", "base": "region"}"#,
        r#"{"field": "region", "base": "amount", "grain": "day"}"#,
    );
    assert_eq!(codes(&model), ["E0C-53", "E0C-57"]);
}

#[test]
fn sum_6_1_7_a_metric_that_refers_to_a_summary_gets_no_type() {
    let typing = typing_of(
        conformance_model("e0-summary-metric-001"),
        CATALOG,
        OssieFormat::Release,
    );
    let daily_amount = "/semantic_model/0/metrics/2";
    assert!(typing.warnings.is_empty());
    assert!(typing.unusable.is_empty());
    assert!(
        typing
            .elements
            .iter()
            .all(|element| element.pointer.to_string() != daily_amount)
    );
    assert!(typing.elements.iter().any(|element| {
        element.kind == ElementKind::Metric
            && element.pointer.to_string() == "/semantic_model/0/metrics/0"
    }));
}

#[test]
fn sum_6_1_7_no_e0f_diagnostic_for_a_reference_to_a_summary() {
    let model = conformance_model("e0-summary-metric-001").replacen(
        "SUM(orders_daily.total_amount)",
        "SUM(orders_daily.region)",
        1,
    );
    let typing = typing_of(&model, CATALOG, OssieFormat::Release);
    assert!(typing.warnings.is_empty());
}

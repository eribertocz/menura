//! The summary extension in E0.d: SUM-4.1, SUM-6.1.2, SUM-6.1.7, `E0C-50`, `E0C-51`, `E0C-54`,
//! and `E0D-04` for a metric that refers to a summary.

use semantic_compiler_contracts::admit_catalog_schema;
use semantic_compiler_contracts::types::audit_record::OssieFormat;

use super::{pair, summary, unusable};
use crate::extensions::summaries::datasets::summary_datasets;
use crate::extensions::summaries::fixture::{
    CATALOG, MODEL, SUMMARY_DATA, conformance_model, with_aggregate, with_changed, with_text,
};
use crate::extensions::{DEFAULT_NAMESPACE, validate_extensions};
use crate::read::read_document;
use crate::references::context::resolve_context_references;
use crate::references::summaries::resolve_summary_references;
use crate::references::{ElementKind, References, resolve_references};
use crate::structure::validate_structure;

/// The codes of every diagnostic of the summary references of `model`, read in `format`, each
/// at the `data` member `data`.
fn codes_in(model: &str, format: OssieFormat, data: &str) -> Vec<&'static str> {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure = validate_structure(&document, format, None).expect("the structure is valid");
    let extensions = validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
        .expect("the extensions are accepted");
    match resolve_summary_references(&structure, &extensions) {
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
    codes_in(model, OssieFormat::Release, SUMMARY_DATA)
}

fn changed_codes(from: &str, to: &str) -> Vec<&'static str> {
    codes(&with_changed(from, to))
}

#[test]
fn sum_6_1_2_the_valid_summaries_resolve() {
    assert!(codes(MODEL).is_empty());
    assert!(
        codes_in(
            conformance_model("e0-summaries-development"),
            OssieFormat::Development,
            "/datasets/1/nonportable_extensions/0/data"
        )
        .is_empty()
    );
}

#[test]
fn sum_6_1_2_names_are_declared_names() {
    assert!(changed_codes(r#""of": "orders""#, r#""of": "ORDERS""#).is_empty());
    assert!(changed_codes(r#""base": "region""#, r#""base": "Region""#).is_empty());
    assert!(
        changed_codes(
            r#""coverage_field": "order_date""#,
            r#""coverage_field": "ORDER_DATE""#
        )
        .is_empty()
    );
    assert_eq!(
        changed_codes(r#""base": "region""#, r#""base": "\"Region\"""#),
        ["E0C-51"]
    );
}

#[test]
fn sum_6_1_2_e0c_50_of_names_no_dataset() {
    assert_eq!(
        changed_codes(r#""of": "orders""#, r#""of": "invoices""#),
        ["E0C-50"]
    );
}

#[test]
fn sum_6_1_2_e0c_50_of_names_a_summary() {
    assert_eq!(
        codes(conformance_model("e0-summary-of-summary-001")),
        ["E0C-50"]
    );
}

#[test]
fn sum_6_1_2_e0c_51_a_missing_base() {
    assert_eq!(
        codes(conformance_model("e0-summary-missing-field-001")),
        ["E0C-51"]
    );
}

#[test]
fn sum_6_1_2_e0c_51_a_missing_summary_field() {
    assert_eq!(
        changed_codes(
            r#"{"field": "region", "base": "region"}"#,
            r#"{"field": "area", "base": "region"}"#
        ),
        ["E0C-51"]
    );
    assert_eq!(
        changed_codes(r#""field": "order_count""#, r#""field": "orders""#),
        ["E0C-51"]
    );
}

#[test]
fn sum_6_1_2_e0c_51_every_summary_field_is_named_once() {
    let unnamed = with_changed(
        r#", {"field": "order_count", "base_aggregate": "COUNT(orders.order_id)"}"#,
        "",
    );
    assert_eq!(codes(&unnamed), ["E0C-51"]);
    assert_eq!(
        changed_codes(r#""field": "order_count""#, r#""field": "region""#),
        ["E0C-51"]
    );
}

#[test]
fn sum_6_1_2_e0c_51_a_missing_reference_in_base_aggregate() {
    assert_eq!(
        changed_codes("SUM(orders.amount)", "SUM(orders.price)"),
        ["E0C-51"]
    );
    assert_eq!(
        changed_codes("SUM(orders.amount)", "SUM(invoices.amount)"),
        ["E0C-51"]
    );
}

#[test]
fn sum_6_1_7_e0c_51_base_aggregate_reads_a_summary() {
    assert_eq!(
        changed_codes("SUM(orders.amount)", "SUM(orders_daily.total_amount)"),
        ["E0C-51"]
    );
}

#[test]
fn sum_6_1_3_e0c_51_leaves_other_base_aggregate_problems_to_e0f() {
    for text in [
        "AVG(orders.amount)",
        "SUM(amount)",
        "SUM(orders.amount",
        "AGG(total_amount)",
    ] {
        assert!(
            changed_codes("SUM(orders.amount)", text).is_empty(),
            "{text}"
        );
    }
}

#[test]
fn sum_6_1_2_bases_are_not_checked_without_the_of_dataset() {
    let aggregate = with_changed(r#""of": "orders""#, r#""of": "invoices""#);
    let aggregate = aggregate.replacen(r#""base": "region""#, r#""base": "area""#, 1);
    assert_eq!(codes(&aggregate), ["E0C-50"]);
}

#[test]
fn sum_6_1_2_e0c_54_the_coverage_field() {
    assert_eq!(
        codes(conformance_model("e0-summary-coverage-001")),
        ["E0C-54"]
    );
    assert_eq!(
        changed_codes(
            r#""coverage_field": "order_date""#,
            r#""coverage_field": "total_amount""#
        ),
        ["E0C-54"]
    );
    assert_eq!(
        changed_codes(
            r#""coverage_field": "order_date""#,
            r#""coverage_field": "shipped_on""#
        ),
        ["E0C-54"]
    );
}

#[test]
fn sum_6_1_2_several_codes_of_one_summary() {
    let model = with_changed(r#""base": "region""#, r#""base": "area""#);
    let model = model.replacen(
        r#""coverage_field": "order_date""#,
        r#""coverage_field": "region""#,
        1,
    );
    assert_eq!(codes(&model), ["E0C-51", "E0C-54"]);
}

/// The references of `model` with the summaries of its raw entries (SUM-6.1.7).
fn references_of(model: &str) -> References {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure =
        validate_structure(&document, OssieFormat::Release, None).expect("the structure is valid");
    let catalog = admit_catalog_schema(CATALOG).expect("the catalog schema is admitted");
    let summaries = summary_datasets(&structure.semantic_model, DEFAULT_NAMESPACE);
    resolve_references(&document, &structure, &catalog, &summaries).expect("the model loads")
}

const DAILY_AMOUNT: &str = "/semantic_model/0/metrics/2";

#[test]
fn sum_4_1_sum_6_1_7_e0d_04_a_metric_that_refers_to_a_summary() {
    let references = references_of(conformance_model("e0-summary-metric-001"));
    assert_eq!(
        summary(&references.warnings),
        [pair("E0D-04", &format!("{DAILY_AMOUNT}/expression"))]
    );
    assert_eq!(
        unusable(&references),
        [(ElementKind::Metric, DAILY_AMOUNT.to_owned(), "E0D-04")]
    );
}

#[test]
fn sum_6_1_7_e0d_04_whatever_e0c_reports() {
    let model = conformance_model("e0-summary-metric-001").replacen(
        r#""aggregate": {"of": "orders","#,
        r#""aggregate": {"of": 5,"#,
        1,
    );
    assert_ne!(model, conformance_model("e0-summary-metric-001"));
    let references = references_of(&model);
    assert_eq!(
        summary(&references.warnings),
        [pair("E0D-04", &format!("{DAILY_AMOUNT}/expression"))]
    );
}

#[test]
fn sum_6_1_7_the_valid_model_has_no_reference_diagnostic() {
    let references = references_of(MODEL);
    assert!(references.warnings.is_empty());
    assert!(references.unusable.is_empty());
}

#[test]
fn sum_6_1_7_e0c_63_a_context_remove_field_of_a_summary() {
    let context = |field: &str| {
        with_text(
            "        expression: {dialects: [{dialect: ANSI_SQL, expression: 'COUNT(orders.order_id)'}]}\n",
            &format!(
                "        expression: {{dialects: [{{dialect: ANSI_SQL, expression: 'COUNT(orders.order_id)'}}]}}\n      - name: amount_share\n        expression: {{dialects: [{{dialect: ANSI_SQL, expression: 'AGG(total_amount)'}}]}}\n        custom_extensions:\n          - vendor_name: COMMON\n            data: '{{\"namespace\": \"semantic_compiler\", \"version\": 1, \"context\": {{\"base\": \"total_amount\", \"modifier\": {{\"type\": \"share_of_total\", \"remove\": [\"{field}\"]}}}}}}'\n"
            ),
        )
    };
    let resolve = |model: &str| {
        let document = read_document(model.as_bytes()).expect("the document is read");
        let structure = validate_structure(&document, OssieFormat::Release, None)
            .expect("the structure is valid");
        let extensions =
            validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
                .expect("the extensions are accepted");
        resolve_context_references(&document, &structure, &extensions)
            .err()
            .unwrap_or_default()
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>()
    };
    assert!(resolve(&context("orders.region")).is_empty());
    assert_eq!(resolve(&context("orders_daily.region")), ["E0C-63"]);
}

#[test]
fn sum_6_1_2_the_aggregate_shape_was_read_in_e0c() {
    assert!(
        codes(&with_aggregate(
            &crate::extensions::summaries::fixture::AGGREGATE.replace("SUM(", "MAX(")
        ))
        .is_empty()
    );
}

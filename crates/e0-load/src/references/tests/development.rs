//! Resolving references in the development-line format (section 4.5, OSS-5.1): the compiled
//! `sql` variant and dataset-scoped metrics give the codes of the release format (E0D-3.1,
//! E0D-3.4, `E0D-03` to `E0D-07`, `E0F-02`).
//!
//! The models of the `e0-references-development-*` conformance cases are read here with
//! `ossie_format: development`, as their `load_options` set (CS-2.7).

use super::*;
use crate::references::ReferenceCode;

fn development(files: (&str, &str)) -> References {
    resolve_with(files, OssieFormat::Development, None).expect("the model loads")
}

/// A development-line document with the dataset `orders`, whose fields are `order_id` and
/// `amount` and whose dataset metrics are `metrics`, in the YAML flow style.
fn with_dataset_metrics(metrics: &str) -> String {
    format!(
        "version: \"0.3.0.dev0\"\nname: shop\ndatasets:\n  - {{name: orders, source: shop.main.orders, \
         primary_key: [order_id], fields: [{{name: order_id, expression: {{sql: order_id}}}}, \
         {{name: amount, expression: {{sql: amount}}}}], metrics: [{metrics}]}}\n"
    )
}

fn dataset_metric(name: &str, sql: &str) -> String {
    format!("{{name: {name}, expression: {{sql: '{sql}'}}}}")
}

#[test]
fn oss_5_1_e0d_3_1_sql_expressions_and_dataset_metrics_give_the_release_codes() {
    let references = development(conformance_model!("e0-references-development-missing"));
    assert_eq!(
        summary(&references.warnings),
        [
            pair("E0D-03", "/datasets/0/fields/4/expression"),
            pair("E0D-04", "/datasets/0/metrics/1/expression"),
            pair("E0D-07", "/datasets/2/primary_key/0"),
            pair("E0D-05", "/relationships/1/to"),
            pair("E0D-06", "/relationships/2/from_columns/0"),
            pair("E0D-04", "/metrics/1/expression"),
            pair("E0D-04", "/metrics/2/expression"),
        ]
    );
    assert!(
        references
            .warnings
            .iter()
            .all(|warning| warning.code.load_code() == ReferenceCode::MISSING_REFERENCE_WARNING)
    );
    let element = |kind, pointer: &str, code| (kind, pointer.to_owned(), code);
    assert_eq!(
        unusable(&references),
        [
            element(ElementKind::Field, "/datasets/0/fields/4", "E0D-03"),
            element(ElementKind::Metric, "/datasets/0/metrics/1", "E0D-04"),
            element(ElementKind::Dataset, "/datasets/2", "E0D-07"),
            element(ElementKind::Relationship, "/relationships/1", "E0D-05"),
            element(ElementKind::Relationship, "/relationships/2", "E0D-06"),
            element(ElementKind::Metric, "/metrics/1", "E0D-04"),
            element(ElementKind::Metric, "/metrics/2", "E0D-04"),
        ]
    );
}

#[test]
fn oss_5_1_e0d_3_4_e0f_02_unlisted_reference_forms_in_sql() {
    let references = development(conformance_model!(
        "e0-references-development-reference-forms"
    ));
    assert_eq!(
        summary(&references.warnings),
        [
            pair("E0F-02", "/datasets/0/fields/3/expression"),
            pair("E0F-02", "/metrics/1/expression"),
        ]
    );
    assert!(
        references
            .warnings
            .iter()
            .all(|warning| warning.code.load_code() == ReferenceCode::EXPRESSION_WARNING)
    );
    assert_eq!(
        unusable(&references),
        [
            (
                ElementKind::Field,
                "/datasets/0/fields/3".to_owned(),
                "E0F-02"
            ),
            (ElementKind::Metric, "/metrics/1".to_owned(), "E0F-02"),
        ]
    );
}

#[test]
fn oss_5_1_a_dataset_metric_reads_the_fields_of_its_dataset_by_bare_name() {
    let metrics = [
        dataset_metric("total", "SUM(amount) + COUNT(*)"),
        dataset_metric("quoted", "SUM(\"AMOUNT\")"),
        dataset_metric("column_only", "SUM(customer_id)"),
    ]
    .join(", ");
    let references = development((&with_dataset_metrics(&metrics), CATALOG));
    assert_eq!(
        summary(&references.warnings),
        [pair("E0D-04", "/datasets/0/metrics/2/expression")]
    );
}

#[test]
fn oss_5_1_qualified_references_in_dataset_metrics_are_left_unchecked() {
    let metrics = [
        dataset_metric("from_source", "SUM(source.discount)"),
        dataset_metric("other_dataset", "SUM(customers.name)"),
    ]
    .join(", ");
    assert_eq!(
        development((&with_dataset_metrics(&metrics), CATALOG)),
        References::default()
    );
}

#[test]
fn oss_5_1_the_compiled_sql_variant_is_the_one_resolved() {
    let document = "version: \"0.3.0.dev0\"\nname: shop\ndatasets:\n  - {name: orders, \
                    source: shop.main.orders, primary_key: [order_id], fields: [\
                    {name: order_id, expression: {sql: order_id}}, \
                    {name: fee, expression: {sql: fee, dialects: [{dialect: ANSI_SQL, expression: amount}]}}]}\n";
    assert_eq!(
        summary(&development((document, CATALOG)).warnings),
        [pair("E0D-03", "/datasets/0/fields/1/expression")]
    );
}

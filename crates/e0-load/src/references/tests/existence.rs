//! Existence of references: E0D-3, `E0D-02` to `E0D-08`, `W0D-01`, `E0F-02`, DEV-3 and the
//! dispositions of E0-4.

use super::*;
use crate::references::ReferenceCode;

#[test]
fn e0d_02_dev_3_a_source_the_catalog_does_not_describe() {
    let rejection = rejected(conformance_model!(
        "e0-references-source-not-in-catalog-001"
    ));
    assert_eq!(
        summary(&rejection.errors),
        [pair("E0D-02", &format!("{ORDERS}/source"))]
    );
}

#[test]
fn e0d_03_to_e0d_07_w0d_01_missing_references_make_elements_unusable() {
    let references = loaded(conformance_model!("e0-references-missing"));
    assert_eq!(
        summary(&references.warnings),
        [
            pair("E0D-03", &format!("{ORDERS}/fields/4/expression")),
            pair("E0D-07", "/semantic_model/0/datasets/2/primary_key/0"),
            pair("E0D-05", "/semantic_model/0/relationships/1/to"),
            pair("E0D-06", "/semantic_model/0/relationships/2/from_columns/0"),
            pair("E0D-04", "/semantic_model/0/metrics/1/expression"),
            pair("E0D-04", "/semantic_model/0/metrics/2/expression"),
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
            element(ElementKind::Field, &format!("{ORDERS}/fields/4"), "E0D-03"),
            element(
                ElementKind::Dataset,
                "/semantic_model/0/datasets/2",
                "E0D-07"
            ),
            element(
                ElementKind::Relationship,
                "/semantic_model/0/relationships/1",
                "E0D-05"
            ),
            element(
                ElementKind::Relationship,
                "/semantic_model/0/relationships/2",
                "E0D-06"
            ),
            element(ElementKind::Metric, "/semantic_model/0/metrics/1", "E0D-04"),
            element(ElementKind::Metric, "/semantic_model/0/metrics/2", "E0D-04"),
        ]
    );
}

#[test]
fn e0d_3_1_a_key_resolves_to_a_field_then_to_a_source_column() {
    let field_only = document(
        &orders("order_key", &field("order_key", "order_id + 1")),
        "",
    );
    assert_eq!(loaded((&field_only, CATALOG)), References::default());
    let column_only = document(&orders("customer_id", ""), "");
    assert_eq!(loaded((&column_only, CATALOG)), References::default());
    let exactly_the_column = document(&orders("\"AMOUNT\"", ""), "");
    assert_eq!(
        loaded((&exactly_the_column, CATALOG)),
        References::default()
    );
    let other_column = document(&orders("status", &field("status", "UPPER(status)")), "");
    assert_eq!(
        summary(&loaded((&other_column, CATALOG)).warnings),
        [pair("E0D-07", &format!("{ORDERS}/primary_key/0"))]
    );
    let neither = document(&orders("order_id, tax", ""), "");
    assert_eq!(
        summary(&loaded((&neither, CATALOG)).warnings),
        [pair("E0D-06", &format!("{ORDERS}/primary_key/1"))]
    );
}

#[test]
fn e0d_3_1_unique_keys_and_both_relationship_ends_are_resolved() {
    let dataset = orders("order_id", "").replace(
        "primary_key: [order_id]",
        "primary_key: [order_id], unique_keys: [[order_id], [amount, tax]]",
    );
    let relationship = "    relationships:\n      - {name: self, from: orders, to: orders, from_columns: [amount], to_columns: [fee]}\n";
    let references = loaded((&document(&dataset, relationship), CATALOG));
    assert_eq!(
        summary(&references.warnings),
        [
            pair("E0D-06", &format!("{ORDERS}/unique_keys/1/1")),
            pair("E0D-06", "/semantic_model/0/relationships/0/to_columns/0"),
        ]
    );
}

#[test]
fn e0d_3_2_references_inside_extensions_get_no_generic_code() {
    let dataset = orders("order_id", "").replace(
        "primary_key: [order_id]",
        "primary_key: [order_id], custom_extensions: [{vendor_name: COMMON, data: '{\"namespace\": \"menura\", \"version\": 1, \"additivity\": {\"over\": \"refunds.tax\"}}'}]",
    );
    assert_eq!(
        loaded((&document(&dataset, ""), CATALOG)),
        References::default()
    );
}

#[test]
fn e0_4_1_an_element_keeps_the_lowest_query_error_of_its_references() {
    let dataset = orders("status, tax", &field("status", "UPPER(status)"));
    let references = loaded((&document(&dataset, ""), CATALOG));
    assert_eq!(
        summary(&references.warnings),
        [
            pair("E0D-07", &format!("{ORDERS}/primary_key/0")),
            pair("E0D-06", &format!("{ORDERS}/primary_key/1")),
        ]
    );
    assert_eq!(
        unusable(&references),
        [(ElementKind::Dataset, ORDERS.to_owned(), "E0D-06")]
    );
}

#[test]
fn e0_4_3_a_rejection_keeps_its_warnings_in_document_order() {
    let datasets = format!(
        "{}, {}",
        orders("order_id", &field("discount", "discount")),
        orders("order_id", "").replace("source: shop.orders", "source: shop.refunds")
    );
    let rejection = rejected((&document(&datasets, ""), CATALOG));
    assert_eq!(
        summary(&rejection.errors),
        [
            pair("E0D-01", "/semantic_model/0/datasets/1/name"),
            pair("E0D-02", "/semantic_model/0/datasets/1/source"),
        ]
    );
    assert_eq!(
        summary(&rejection.warnings),
        [pair("E0D-03", &format!("{ORDERS}/fields/2/expression"))]
    );
}

#[test]
fn elements_whose_expression_cannot_be_used_are_left_to_e0b_and_e0f() {
    let unparsable = field("total", "SUM(");
    let other_dialect =
        ", {name: label, expression: {dialects: [{dialect: SNOWFLAKE, expression: missing}]}}";
    let dataset = orders("label", &format!("{unparsable}{other_dialect}"));
    let metrics = "    metrics: [{name: m, expression: {dialects: [{dialect: ANSI_SQL, expression: 'SUM(missing.x'}]}}]\n";
    assert_eq!(
        loaded((&document(&dataset, metrics), CATALOG)),
        References::default()
    );
}

#[test]
fn e0d_3_3_e0d_08_a_query_source_is_a_single_read_only_select() {
    assert_eq!(
        loaded(conformance_model!("e0-references-query-source")),
        References::default()
    );
    let rejection = rejected(conformance_model!("e0-references-query-source-001"));
    assert_eq!(
        summary(&rejection.errors),
        [pair("E0D-08", "/semantic_model/0/datasets/1/source")]
    );
    assert_eq!(rejection.errors[0].code.load_code(), "E0D-08");
}

#[test]
fn e0d_3_4_e0f_02_unlisted_reference_forms_make_elements_unusable() {
    let references = loaded(conformance_model!("e0-references-reference-forms"));
    assert_eq!(
        summary(&references.warnings),
        [
            pair("E0F-02", &format!("{ORDERS}/fields/3/expression")),
            pair("E0F-02", "/semantic_model/0/metrics/1/expression"),
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
            (ElementKind::Field, format!("{ORDERS}/fields/3"), "E0F-02"),
            (
                ElementKind::Metric,
                "/semantic_model/0/metrics/1".to_owned(),
                "E0F-02"
            ),
        ]
    );
}

#[test]
fn e0d_3_4_a_missing_column_comes_before_an_unlisted_form() {
    let dataset = orders("order_id", &field("both", "orders.amount + discount"));
    let references = loaded((&document(&dataset, ""), CATALOG));
    assert_eq!(
        summary(&references.warnings),
        [
            pair("E0D-03", &format!("{ORDERS}/fields/2/expression")),
            pair("E0F-02", &format!("{ORDERS}/fields/2/expression")),
        ]
    );
    assert_eq!(
        unusable(&references),
        [(ElementKind::Field, format!("{ORDERS}/fields/2"), "E0D-03")]
    );
}

#[test]
fn e6_4_3_a_relation_name_that_starts_with_with_is_not_a_query() {
    assert_eq!(
        loaded(conformance_model!(
            "e0-references-relation-named-like-keyword"
        )),
        References::default()
    );
}

//! Resolving references: E0D-1.1 to E0D-1.3, E0D-3.1, E0D-3.2, E0D-01 to E0D-07, W0D-01,
//! DEV-2, DEV-3 and the dispositions of E0-4.
//!
//! The model documents and catalog schemas of the `e0-references-*` conformance cases are read
//! here at compile time, so reference resolution is checked against the files the conformance
//! suite uses.

use semantic_compiler_contracts::admit_catalog_schema;

use super::diagnostic::ReferenceDiagnostic;
use super::{ElementKind, ReferenceCode, ReferenceRejection, References, resolve_references};
use crate::read::read_document;
use crate::structure::validate_structure;

macro_rules! conformance_file {
    ($name:literal, $file:literal) => {
        include_str!(concat!(
            "../../../../conformance/models/",
            $name,
            "/",
            $file
        ))
    };
}

macro_rules! conformance_model {
    ($name:literal) => {
        (
            conformance_file!($name, "model.yaml"),
            conformance_file!($name, "catalog-schema.json"),
        )
    };
}

/// The catalog schema of the `e0-references-missing` model: `shop.main.orders`
/// (`order_id`, `customer_id`, `status`, `amount`), `shop.main.customers` (`customer_id`,
/// `name`) and `shop.main.labels` (`label_id`, `name`).
const CATALOG: &str = conformance_file!("e0-references-missing", "catalog-schema.json");

fn resolve((model, catalog): (&str, &str)) -> Result<References, ReferenceRejection> {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure = validate_structure(&document, None).expect("the structure is valid");
    let catalog = admit_catalog_schema(catalog).expect("the catalog schema is admitted");
    resolve_references(&document, &structure, &catalog)
}

fn loaded(files: (&str, &str)) -> References {
    resolve(files).expect("the model loads")
}

fn rejected(files: (&str, &str)) -> ReferenceRejection {
    resolve(files).expect_err("the model is rejected")
}

fn pair(code: &'static str, pointer: &str) -> (&'static str, String) {
    (code, pointer.to_owned())
}

fn summary(diagnostics: &[ReferenceDiagnostic]) -> Vec<(&'static str, String)> {
    diagnostics
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect()
}

fn unusable(references: &References) -> Vec<(ElementKind, String, &'static str)> {
    references
        .unusable
        .iter()
        .map(|element| {
            (
                element.kind,
                element.pointer.to_string(),
                element.query_error.as_str(),
            )
        })
        .collect()
}

/// A document with one model whose datasets are `datasets` and whose other members are `rest`,
/// in the YAML flow style.
fn document(datasets: &str, rest: &str) -> String {
    format!(
        "version: \"0.1.1\"\nsemantic_model:\n  - name: shop\n    datasets: [{datasets}]\n{rest}"
    )
}

/// The dataset `orders` with the fields `order_id` and `amount` and the primary key `keys`.
fn orders(keys: &str, extra_fields: &str) -> String {
    format!(
        "{{name: orders, source: shop.main.orders, primary_key: [{keys}], fields: [\
         {{name: order_id, expression: {{dialects: [{{dialect: ANSI_SQL, expression: order_id}}]}}}},\
         {{name: amount, expression: {{dialects: [{{dialect: ANSI_SQL, expression: amount}}]}}}}{extra_fields}]}}"
    )
}

fn field(name: &str, expression: &str) -> String {
    format!(
        ", {{name: {name}, expression: {{dialects: [{{dialect: ANSI_SQL, expression: '{expression}'}}]}}}}"
    )
}

const ORDERS: &str = "/semantic_model/0/datasets/0";

#[test]
fn e0d_1_1_e0d_1_3_dev_2_names_that_differ_only_in_case_collide() {
    let rejection = rejected(conformance_model!("e0-references-duplicate-name-001"));
    assert_eq!(
        summary(&rejection.errors),
        [pair("E0D-01", "/semantic_model/0/datasets/1/name")]
    );
    assert_eq!(rejection.errors[0].code.load_code(), "E0D-01");
    assert!(rejection.warnings.is_empty());
}

#[test]
fn e0d_1_2_e0d_01_a_name_longer_than_128_characters() {
    let rejection = rejected(conformance_model!("e0-references-long-name-001"));
    assert_eq!(
        summary(&rejection.errors),
        [pair("E0D-01", &format!("{ORDERS}/fields/4/name"))]
    );
}

#[test]
fn e0d_1_3_each_kind_of_element_has_its_own_scope() {
    let references = loaded(conformance_model!("e0-references-identifiers"));
    assert_eq!(
        summary(&references.warnings),
        [pair("E0D-04", "/semantic_model/0/metrics/2/expression")]
    );
    let fields = document(
        &format!(
            "{}, {}",
            orders("order_id", ""),
            orders("order_id", "").replace("{name: orders", "{name: others")
        ),
        "    metrics: [{name: amount, expression: {dialects: [{dialect: ANSI_SQL, expression: SUM(orders.amount)}]}}]\n",
    );
    assert_eq!(loaded((&fields, CATALOG)), References::default());
    let repeated = document(&orders("order_id", &field("Amount", "amount")), "");
    assert_eq!(
        summary(&rejected((&repeated, CATALOG)).errors),
        [pair("E0D-01", &format!("{ORDERS}/fields/2/name"))]
    );
}

#[test]
fn e0d_1_1_quoted_names_compare_exactly_and_unquoted_names_in_upper_case() {
    let references = loaded(conformance_model!("e0-references-identifiers"));
    assert_eq!(
        unusable(&references),
        [(
            ElementKind::Metric,
            "/semantic_model/0/metrics/2".to_owned(),
            "E0D-04"
        )]
    );
    assert_eq!(references.warnings[0].code.load_code(), "W0D-01");
    assert!(references.warnings[0].message.contains("`orders`.`amount`"));
}

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
        "primary_key: [order_id], custom_extensions: [{vendor_name: COMMON, data: '{\"namespace\": \"semantic_compiler\", \"version\": 1, \"additivity\": {\"over\": \"refunds.tax\"}}'}]",
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
        orders("order_id", "").replace("source: shop.main.orders", "source: shop.main.refunds")
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

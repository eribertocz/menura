//! Typing: E0F-3, E0F-4.1, E0F-5, E0F-5a, E0F-6.1, E0F-7.1, `E0F-03` to `E0F-08`, `W0F-01`,
//! CAT-4.3 and E0E-3.7.
//!
//! The model document and catalog schema of the `e0-typing` conformance model are read here at
//! compile time, so typing is checked against the files the conformance suite uses.

use semantic_compiler_contracts::admit_catalog_schema;
use semantic_compiler_contracts::types::audit_record::OssieFormat;

use super::diagnostic::TypingDiagnostic;
use super::value_type::Typed;
use super::{ElementKind, Typing, type_expressions};
use crate::read::read_document;
use crate::references::derived::DerivedMetrics;
use crate::structure::validate_structure;

mod additivity;
mod codes;
mod context;
mod derived;
mod development;
mod populations;
mod results;
mod security;

const MODEL: &str = include_str!("../../../../../conformance/models/e0-typing/model.yaml");

/// The catalog schema of the `e0-typing` model: `shop.main.orders` (`order_id` 32-bit,
/// `customer_id` 64-bit, `status` text, `quantity` 32-bit, `price` `decimal(10,2)`, `rate`
/// `decimal(30,20)`, `discount` nullable 64-bit float, `payload` unsupported, `ordered_on`
/// date, `ordered_at` nullable timestamp, `note` nullable text) and `shop.main.customers`
/// (`customer_id` 32-bit, `customer_code` text, `name` text).
const CATALOG: &str =
    include_str!("../../../../../conformance/models/e0-typing/catalog-schema.json");

const ORDERS: &str = "/semantic_model/0/datasets/0";

fn type_document(model: &str) -> Typing {
    type_in_format(model, CATALOG, OssieFormat::Release)
}

/// Types `model`, read in `format`, against the catalog schema `catalog`.
fn type_in_format(model: &str, catalog: &str, format: OssieFormat) -> Typing {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure = validate_structure(&document, format, None).expect("the structure is valid");
    let catalog = admit_catalog_schema(catalog).expect("the catalog schema is admitted");
    type_expressions(&document, &structure, &catalog, &DerivedMetrics::default())
}

fn pair(code: &'static str, pointer: &str) -> (&'static str, String) {
    (code, pointer.to_owned())
}

fn summary(diagnostics: &[TypingDiagnostic]) -> Vec<(&'static str, String)> {
    diagnostics
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect()
}

/// What typing found for the probe element.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Probe {
    /// The element is typed.
    Typed(Typed),
    /// The element is unusable with this code.
    Problem(&'static str),
    /// The element has no type and no diagnostic.
    Untyped,
}

/// One-line YAML for `text`, single-quoted.
fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// A document whose dataset `orders` reads every column of `shop.main.orders` with a field of
/// the same name, and has the field `probe` with `field`; the metric `probe` has `metric`.
fn document(field: &str, metric: &str) -> String {
    let columns = [
        "order_id",
        "customer_id",
        "status",
        "quantity",
        "price",
        "rate",
        "discount",
        "payload",
        "ordered_on",
        "ordered_at",
        "note",
    ];
    let mut fields: Vec<String> = columns
        .iter()
        .map(|column| element(column, column))
        .collect();
    fields.push(element("probe", field));
    format!(
        "version: \"0.1.1\"\nsemantic_model:\n  - name: shop\n    datasets:\n      - name: orders\n        \
         source: shop.main.orders\n        fields: [{}]\n    metrics: [{}]\n",
        fields.join(", "),
        element("probe", metric)
    )
}

fn element(name: &str, expression: &str) -> String {
    format!(
        "{{name: {name}, expression: {{dialects: [{{dialect: ANSI_SQL, expression: {}}}]}}}}",
        quoted(expression)
    )
}

fn probe(typing: &Typing, kind: ElementKind, pointer: &str) -> Probe {
    if let Some(element) = typing
        .elements
        .iter()
        .find(|element| element.kind == kind && element.pointer.as_str() == pointer)
    {
        return Probe::Typed(element.typed.clone());
    }
    match typing
        .unusable
        .iter()
        .find(|element| element.pointer.as_str() == pointer)
    {
        Some(element) => Probe::Problem(element.query_error.as_str()),
        None => Probe::Untyped,
    }
}

/// What typing finds for a field with the expression `expression`.
fn field(expression: &str) -> Probe {
    let typing = type_document(&document(expression, "COUNT(*)"));
    probe(&typing, ElementKind::Field, &format!("{ORDERS}/fields/11"))
}

/// What typing finds for a metric with the expression `expression`.
fn metric(expression: &str) -> Probe {
    let typing = type_document(&document("order_id", expression));
    probe(&typing, ElementKind::Metric, "/semantic_model/0/metrics/0")
}

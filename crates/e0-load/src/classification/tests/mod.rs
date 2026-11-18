//! Metric classification: E0G-1.1 to E0G-1.4, E0G-2.1 to E0G-2.8, E0G-3.1, E0G-3.2, E0G-4,
//! E0G-6, `E0G-01`, `E0G-02` and `W0G-01`.
//!
//! The model documents and catalog schemas of the `e0-metrics` and `e0-metrics-development`
//! conformance models are read here at compile time, so classification is checked against the
//! files the conformance suite uses.

use semantic_compiler_contracts::admit_catalog_schema;
use semantic_compiler_contracts::types::audit_record::OssieFormat;

use super::diagnostic::ClassificationCode;
use super::output::{Additivity, DecomposabilityClass, QueryError};
use super::{Classification, ClassificationInput, ClassifiedMetric, MetricKind, classify_metrics};
use crate::extensions::{DEFAULT_NAMESPACE, validate_extensions};
use crate::names::Name;
use crate::read::read_document;
use crate::references::context::resolve_context_references;
use crate::references::derived::resolve_derived_references;
use crate::references::resolve_references;
use crate::structure::validate_structure;
use crate::typing::derived::type_derived_metrics;
use crate::typing::type_expressions;

mod additivity;
mod classes;
mod context;
mod derived;
mod development;
mod rules;

const MODEL: &str = include_str!("../../../../../conformance/models/e0-metrics/model.yaml");

/// The catalog schema of the `e0-metrics` models: `shop.main.orders` (`order_id`,
/// `customer_id`, `status` text, `amount`) and `shop.main.customers` (`customer_id`, `region`
/// text, `age`); every integer is 32-bit and no column is nullable.
const CATALOG: &str =
    include_str!("../../../../../conformance/models/e0-metrics/catalog-schema.json");

/// Classifies `model`, read in `format`, after typing it against `catalog`.
fn classify_in_format(model: &str, catalog: &str, format: OssieFormat) -> Classification {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure = validate_structure(&document, format, None).expect("the structure is valid");
    let catalog = admit_catalog_schema(catalog).expect("the catalog schema is admitted");
    let extensions = validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
        .expect("the extensions are accepted");
    let references =
        resolve_references(&document, &structure, &catalog).expect("the references are accepted");
    let derived = resolve_derived_references(&document, &structure, &extensions)
        .expect("the inputs of derived metrics resolve");
    let context = resolve_context_references(&document, &structure, &extensions)
        .expect("the bases of context metrics resolve");
    let typing = type_expressions(&document, &structure, &catalog, &derived);
    let derived_typing =
        type_derived_metrics(&document, &structure, &catalog, &typing, &derived, &context)
            .expect("the derived formulas are accepted");
    let input = ClassificationInput {
        extensions: &extensions,
        references: &references,
        derived: &derived,
        context: &context,
        typing: &typing,
        derived_typing: &derived_typing,
    };
    classify_metrics(&document, &structure, &input)
}

fn classify(model: &str) -> Classification {
    classify_in_format(model, CATALOG, OssieFormat::Release)
}

/// The code and pointer of each warning.
fn summary(classification: &Classification) -> Vec<(&'static str, String)> {
    classification
        .warnings
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect()
}

fn pair(code: ClassificationCode, pointer: &str) -> (&'static str, String) {
    (code.as_str(), pointer.to_owned())
}

/// What classification found for one metric.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Probe {
    /// The metric is classified.
    Classified(Box<ClassifiedMetric>),
    /// The metric is unusable with this code of E0.g.
    Unusable(ClassificationCode),
    /// The metric is unusable with the query error of an unusable input (DER-5.1.5).
    UnusableInput(QueryError),
    /// The metric is neither: it is not usable after E0.f.
    NotClassified,
}

fn probe(classification: &Classification, pointer: &str) -> Probe {
    if let Some(metric) = classification
        .metrics
        .iter()
        .find(|metric| metric.pointer.as_str() == pointer)
    {
        return Probe::Classified(Box::new(metric.clone()));
    }
    match classification
        .unusable
        .iter()
        .find(|metric| metric.pointer.as_str() == pointer)
    {
        Some(metric) => match metric.query_error {
            QueryError::Classification(code) => Probe::Unusable(code),
            other => Probe::UnusableInput(other),
        },
        None => Probe::NotClassified,
    }
}

/// A document with the datasets `orders` and `customers` of the `e0-metrics` model and one
/// metric, `probe`, with `expression`.
fn document(expression: &str) -> String {
    let field = |name: &str| {
        format!(
            "{{name: {name}, expression: {{dialects: [{{dialect: ANSI_SQL, expression: {name}}}]}}}}"
        )
    };
    let fields = |names: &[&str]| {
        names
            .iter()
            .map(|name| field(name))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "version: \"0.1.1\"\nsemantic_model:\n  - name: shop\n    datasets:\n      - name: orders\n        \
         source: shop.main.orders\n        fields: [{}]\n      - name: customers\n        \
         source: shop.main.customers\n        fields: [{}]\n    metrics:\n      - name: probe\n        \
         expression: {{dialects: [{{dialect: ANSI_SQL, expression: '{}'}}]}}\n",
        fields(&["order_id", "customer_id", "status", "amount"]),
        fields(&["customer_id", "region", "age"]),
        expression.replace('\'', "''")
    )
}

/// What classification finds for a model metric with `expression`.
fn metric(expression: &str) -> Probe {
    probe(
        &classify(&document(expression)),
        "/semantic_model/0/metrics/0",
    )
}

/// The classified metric with `expression`.
fn classified(expression: &str) -> ClassifiedMetric {
    match metric(expression) {
        Probe::Classified(metric) => *metric,
        other => panic!("`{expression}` must be classified, found {other:?}"),
    }
}

/// The class and home dataset of each component of a metric.
fn components(metric: &ClassifiedMetric) -> Vec<(DecomposabilityClass, Name)> {
    metric
        .components
        .iter()
        .map(|component| (component.class, component.home_dataset.clone()))
        .collect()
}

fn orders() -> Name {
    Name::regular("orders")
}

fn customers() -> Name {
    Name::regular("customers")
}

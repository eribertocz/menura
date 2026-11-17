//! Resolving references: E0D-1.1 to E0D-1.3, E0D-3.1, E0D-3.2, E0D-01 to E0D-07, W0D-01,
//! DEV-2, DEV-3 and the dispositions of E0-4.
//!
//! The model documents and catalog schemas of the `e0-references-*` conformance cases are read
//! here at compile time, so reference resolution is checked against the files the conformance
//! suite uses.

use semantic_compiler_contracts::admit_catalog_schema;
use semantic_compiler_contracts::types::audit_record::OssieFormat;

use super::diagnostic::ReferenceDiagnostic;
use super::{ElementKind, ReferenceRejection, References, resolve_references};
use crate::read::read_document;
use crate::structure::validate_structure;

macro_rules! conformance_file {
    ($name:literal, $file:literal) => {
        include_str!(concat!(
            "../../../../../conformance/models/",
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

fn resolve(files: (&str, &str)) -> Result<References, ReferenceRejection> {
    resolve_selecting(files, None)
}

/// Resolves with the load option `model_name`.
fn resolve_selecting(
    (model, catalog): (&str, &str),
    model_name: Option<&str>,
) -> Result<References, ReferenceRejection> {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure = validate_structure(&document, OssieFormat::Release, model_name)
        .expect("the structure is valid");
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

mod existence;
mod names;

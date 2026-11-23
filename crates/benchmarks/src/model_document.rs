//! Writes a generated model as a model document in YAML (R4, R5).

use crate::datasets::{ColumnKind, Dataset, Field, chain_name, datasets, star_name};
use crate::metrics::{Metric, metrics};
use crate::model_shape::ModelShape;

/// The model document of `shape`, as bytes. The same shape always gives the same bytes (R4).
#[must_use]
pub fn model_document(shape: &ModelShape) -> Vec<u8> {
    let mut text = header(shape);
    text.push_str("    datasets:\n");
    for dataset in datasets(shape) {
        text.push_str(&dataset_text(&dataset));
    }
    text.push_str("    relationships:\n");
    for (from, to) in relationships(shape) {
        text.push_str(&relationship_text(&from, &to));
    }
    text.push_str("    metrics:\n");
    for metric in metrics(shape) {
        text.push_str(&metric_text(&metric));
    }
    text.into_bytes()
}

/// YAML single-quoted scalar of `text`.
fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

fn header(shape: &ModelShape) -> String {
    let security = r#"{"namespace": "menura", "version": 1, "security_context": {"roles": ["analyst"], "attributes": [{"name": "regions", "type": "list<text>"}]}}"#;
    format!(
        "version: \"0.1.1\"\nsemantic_model:\n  - name: {}\n    description: 'A generated benchmark model (R5)'\n    custom_extensions:\n      - vendor_name: COMMON\n        data: {}\n",
        shape.name,
        quoted(security)
    )
}

fn expression(text: &str) -> String {
    format!(
        "{{dialects: [{{dialect: ANSI_SQL, expression: {}}}]}}",
        quoted(text)
    )
}

fn extensions_text(indent: &str, extension: &str) -> String {
    format!(
        "{indent}custom_extensions:\n{indent}  - vendor_name: COMMON\n{indent}    data: {}\n",
        quoted(extension)
    )
}

fn field_text(field: &Field) -> String {
    let dimension = if field.kind == ColumnKind::Date {
        "            dimension: {is_time: true}\n"
    } else {
        ""
    };
    format!(
        "          - name: {}\n            expression: {}\n{dimension}",
        field.name,
        expression(&field.name)
    )
}

fn dataset_text(dataset: &Dataset) -> String {
    let fields: String = dataset.fields.iter().map(field_text).collect();
    format!(
        "      - name: {name}\n        source: bench.main.{name}\n        primary_key: [{}]\n        fields:\n{fields}{}",
        dataset.primary_key.join(", "),
        extensions_text("        ", &dataset.extension),
        name = dataset.name
    )
}

/// Every many-to-one relationship: `sales` to each star dataset, then the chain.
fn relationships(shape: &ModelShape) -> Vec<(String, String)> {
    let mut all: Vec<(String, String)> = (1..=shape.star_datasets)
        .map(|index| ("sales".to_owned(), star_name(index)))
        .collect();
    if shape.chain_datasets > 0 {
        all.push(("sales".to_owned(), chain_name(1)));
    }
    all.extend((1..shape.chain_datasets).map(|index| (chain_name(index), chain_name(index + 1))));
    all
}

fn relationship_text(from: &str, to: &str) -> String {
    format!(
        "      - name: {from}_{to}\n        from: {from}\n        to: {to}\n        from_columns: [{to}_id]\n        to_columns: [{to}_id]\n"
    )
}

fn metric_text(metric: &Metric) -> String {
    let extensions = metric
        .extension
        .as_deref()
        .map(|extension| extensions_text("        ", extension))
        .unwrap_or_default();
    format!(
        "      - name: {}\n        expression: {}\n{extensions}",
        metric.name,
        expression(&metric.expression)
    )
}

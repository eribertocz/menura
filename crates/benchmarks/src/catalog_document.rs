//! Writes the catalog schema document of a generated model (R4, R5).

use crate::datasets::{ColumnKind, Dataset, datasets};
use crate::model_shape::ModelShape;

/// The catalog schema document of `shape`, as bytes: one relation per dataset, one column per
/// field, sorted by source. The same shape always gives the same bytes (R4).
#[must_use]
pub fn catalog_schema_document(shape: &ModelShape) -> Vec<u8> {
    let mut all = datasets(shape);
    all.sort_by(|left, right| left.name.cmp(&right.name));
    let relations: Vec<String> = all.iter().map(relation).collect();
    format!(
        r#"{{"contract": "menura.catalog_schema", "contract_version": "1.1.0", "document_id": {{"origin": "benchmarks:{name}", "snapshot": "{name}"}}, "engine": {{"name": "duckdb", "version": "2.0.0"}}, "relations": [{}]}}"#,
        relations.join(", "),
        name = shape.name
    )
    .into_bytes()
}

fn logical_type(kind: ColumnKind) -> (&'static str, &'static str) {
    match kind {
        ColumnKind::Integer => (
            r#"{"kind": "integer", "bits": 32, "signed": true}"#,
            "INTEGER",
        ),
        ColumnKind::Date => (r#"{"kind": "date"}"#, "DATE"),
        ColumnKind::Text => (r#"{"kind": "text"}"#, "VARCHAR"),
    }
}

fn relation(dataset: &Dataset) -> String {
    let columns: Vec<String> = dataset
        .fields
        .iter()
        .map(|field| {
            let (logical, physical) = logical_type(field.kind);
            format!(
                r#"{{"name": "{}", "logical_type": {logical}, "physical_type": "{physical}", "nullable": false}}"#,
                field.name
            )
        })
        .collect();
    format!(
        r#"{{"source": "bench.main.{}", "columns": [{}]}}"#,
        dataset.name,
        columns.join(", ")
    )
}

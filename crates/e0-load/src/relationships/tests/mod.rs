//! Analyzing relationships: E0E-2.1 to E0E-2.4, E0E-3.1 to E0E-3.6, E0E-3.8, E0E-4.1, E0E-4.2,
//! E0E-5.1, E0E-01, E0E-02, W0E-01 to W0E-04, and the dispositions of E0-4.
//!
//! The model documents and catalog schemas of the `e0-relationships-*` conformance cases are
//! read here at compile time, so the analysis is checked against the files the conformance suite
//! uses. The path counts are also checked against an enumeration of every path on small graphs.

use semantic_compiler_contracts::admit_catalog_schema;
use semantic_compiler_contracts::types::audit_record::OssieFormat;

use super::{RelationshipDiagnostic, Relationships, analyze_relationships};
use crate::extensions::{DEFAULT_NAMESPACE, validate_extensions};
use crate::pointer::{JsonPointer, resolve};
use crate::read::{Location, Node, read_document};
use crate::references::resolve_references;
use crate::structure::validate_structure;

macro_rules! conformance_model {
    ($name:literal) => {
        (
            include_str!(concat!(
                "../../../../../conformance/models/",
                $name,
                "/model.yaml"
            )),
            include_str!(concat!(
                "../../../../../conformance/models/",
                $name,
                "/catalog-schema.json"
            )),
        )
    };
}

/// The model of the `e0-relationships-*` conformance cases.
const RELATIONSHIPS: (&str, &str) = conformance_model!("e0-relationships");

/// The model of the `e0-references-*` conformance cases with missing references.
const REFERENCES_MISSING: (&str, &str) = conformance_model!("e0-references-missing");

/// Runs E0.a to E0.e on a model that loads.
fn analyze(files: (&str, &str)) -> Relationships {
    analyze_with_document(files).1
}

/// Runs E0.a to E0.e on a model that loads, and returns the data tree with the result.
fn analyze_with_document((model, catalog): (&str, &str)) -> (Node, Relationships) {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure =
        validate_structure(&document, OssieFormat::Release, None).expect("the structure is valid");
    let extensions = validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
        .expect("the extensions are accepted");
    let catalog = admit_catalog_schema(catalog).expect("the catalog schema is admitted");
    let references =
        resolve_references(&document, &structure, &catalog).expect("the references resolve");
    let relationships = analyze_relationships(
        &document,
        &structure.semantic_model,
        &extensions,
        &references,
    );
    (document, relationships)
}

/// Where the node at `pointer` starts in `document`.
fn node_location(document: &Node, pointer: &str) -> Location {
    resolve(document, &JsonPointer::from_text(pointer))
        .expect("the pointer names a node")
        .location
}

/// The load code, code and JSON Pointer of every diagnostic.
fn summary(diagnostics: &[RelationshipDiagnostic]) -> Vec<(&'static str, &'static str, String)> {
    diagnostics
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.code.load_code(),
                diagnostic.code.as_str(),
                diagnostic.pointer.to_string(),
            )
        })
        .collect()
}

/// The JSON Pointer of the relationship at `index` of the conformance model.
fn relationship(index: usize) -> String {
    format!("/semantic_model/0/relationships/{index}")
}

/// The JSON Pointer of the dataset at `index` of the conformance model.
fn dataset(index: usize) -> String {
    format!("/semantic_model/0/datasets/{index}")
}

/// The normalized names of the datasets of the graph, by index.
fn dataset_names(relationships: &Relationships) -> Vec<&str> {
    relationships
        .graph
        .datasets
        .iter()
        .map(|dataset| dataset.name.as_str())
        .collect()
}

/// The index of the dataset named `name` (normalized) in the graph.
fn index_of(relationships: &Relationships, name: &str) -> usize {
    dataset_names(relationships)
        .iter()
        .position(|dataset| *dataset == name)
        .expect("the dataset is a node")
}

mod additivity;
mod keys;
mod model;
mod paths;

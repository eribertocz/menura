//! E0.h: E0H-1.1 to E0H-1.5, E0H-2, E0H-2.1 to E0H-2.3, E0H-3.1 to E0H-3.4, E0H-4.3,
//! MDL-3.1, CHK-3 and E0-4.2a.
//!
//! The model documents and catalog schemas of the `e0-internal-model`, `e0-additivity` and
//! `e0-summaries` conformance models are read here at compile time, so the internal model is
//! checked against the files the conformance suite uses.

mod checks;
mod content;
mod hashes;
mod order;
mod propagation;
mod row_access_routes;

use menura_contracts::admit_catalog_schema;
use menura_contracts::types::audit_record::OssieFormat;
use menura_model::internal_model::{InternalModel, NormalizedName};

use super::{ModelInput, build_internal_model};
use crate::classification::{ClassificationInput, classify_metrics};
use crate::extensions::security::resolution::resolve_access;
use crate::extensions::summaries::datasets::summary_datasets;
use crate::extensions::{DEFAULT_NAMESPACE, validate_extensions};
use crate::read::read_document;
use crate::references::context::resolve_context_references;
use crate::references::derived::resolve_derived_references;
use crate::references::resolve_references;
use crate::relationships::analyze_relationships;
use crate::structure::validate_structure;
use crate::typing::derived::type_derived_metrics;
use crate::typing::type_expressions;

const MODEL: &str = include_str!("../../../../../conformance/models/e0-internal-model/model.yaml");
const CATALOG: &str =
    include_str!("../../../../../conformance/models/e0-internal-model/catalog-schema.json");
const ADDITIVITY_MODEL: &str =
    include_str!("../../../../../conformance/models/e0-additivity/model.yaml");
const ADDITIVITY_CATALOG: &str =
    include_str!("../../../../../conformance/models/e0-additivity/catalog-schema.json");
/// The `e0-additivity-development-unreachable-001` model reads in the development format; its
/// `customers` dataset metric is removed, so it loads with one semi-additive dataset metric,
/// `debt.debt_balance`, in a dataset that reaches `over` and `entity`.
const DEVELOPMENT_MODEL: &str = include_str!(
    "../../../../../conformance/models/e0-additivity-development-unreachable-001/model.yaml"
);
const DEVELOPMENT_CATALOG: &str = include_str!(
    "../../../../../conformance/models/e0-additivity-development-unreachable-001/catalog-schema.json"
);
const SUMMARIES_MODEL: &str =
    include_str!("../../../../../conformance/models/e0-summaries/model.yaml");
const SUMMARIES_CATALOG: &str =
    include_str!("../../../../../conformance/models/e0-summaries/catalog-schema.json");

/// The internal model of `model`, read in the release format, against `catalog`, after every
/// substage of E0.
fn load(model: &str, catalog: &str) -> InternalModel {
    load_in_format(model, catalog, OssieFormat::Release)
}

/// The internal model of `model`, read in `format`, against `catalog`, after every substage of
/// E0.
fn load_in_format(model: &str, catalog: &str, format: OssieFormat) -> InternalModel {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure = validate_structure(&document, format, None).expect("the structure is valid");
    let catalog = admit_catalog_schema(catalog).expect("the catalog schema is admitted");
    let extensions = validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
        .expect("the extensions are accepted");
    let summaries = summary_datasets(&structure.semantic_model, DEFAULT_NAMESPACE);
    let model = &structure.semantic_model;
    let access = resolve_access(&document, model, &extensions.security, &summaries, false);
    let references = resolve_references(&document, &structure, &catalog, &summaries)
        .expect("the references are accepted");
    let derived = resolve_derived_references(&document, &structure, &extensions)
        .expect("the inputs of derived metrics resolve");
    let context = resolve_context_references(&document, &structure, &extensions)
        .expect("the bases of context metrics resolve");
    let relationships = analyze_relationships(
        &document,
        &structure.semantic_model,
        &extensions,
        &references,
    );
    let typing = type_expressions(&document, &structure, &catalog, &derived, &summaries);
    let derived_typing =
        type_derived_metrics(&document, &structure, &catalog, &typing, &derived, &context)
            .expect("the derived formulas are accepted");
    let earlier = ClassificationInput {
        extensions: &extensions,
        references: &references,
        derived: &derived,
        context: &context,
        typing: &typing,
        derived_typing: &derived_typing,
    };
    let classification = classify_metrics(&document, &structure, &earlier);
    let input = ModelInput {
        structure: &structure,
        extensions: &extensions,
        access: &access,
        references: &references,
        derived: &derived,
        context: &context,
        relationships: &relationships,
        typing: &typing,
        derived_typing: &derived_typing,
        classification: &classification,
    };
    build_internal_model(&document, &input)
}

/// The internal model of the `e0-internal-model` conformance model.
fn fixture() -> InternalModel {
    load(MODEL, CATALOG)
}

/// The model hash of `model` against the `e0-internal-model` catalog schema.
fn hash_of(model: &str) -> String {
    load(model, CATALOG).hash().as_str().to_owned()
}

/// `MODEL` with `from` replaced by `to`, which must occur in it.
fn edited(from: &str, to: &str) -> String {
    assert!(MODEL.contains(from), "the model contains `{from}`");
    MODEL.replacen(from, to, 1)
}

/// The normalized name of the regular identifier `text`.
fn name(text: &str) -> NormalizedName {
    NormalizedName::new(&text.to_uppercase())
}

//! Substage E0.h: builds the immutable internal model from the outputs of E0.b to E0.g
//! (E0H-1, E0H-2), with its hashes (E0H-3) and its recorded integrity checks (E0H-4.3).
//!
//! E0.h reports no diagnostic. It computes the unusability that propagates from one element
//! to the elements that read it (E0-4.2a), removes the relationships made unusable after E0.e
//! from the graph and recomputes reachability with the E0.e algorithm (E0H-2.3).

use menura_model::internal_model::{
    Dataset, ElementKind, InternalModel, ModelParts, NormalizedName, Summary, UnusableElement,
};

use crate::classification::Classification;
use crate::extensions::Extensions;
use crate::extensions::security::resolution::ResolvedAccess;
use crate::read::Node;
use crate::references::References;
use crate::references::context::ContextMetrics;
use crate::references::derived::DerivedMetrics;
use crate::relationships::Relationships;
use crate::structure::Structure;
use crate::typing::Typing;

mod assembly;
mod checks;
mod datasets;
mod declarations;
mod documents;
mod graph;
pub(crate) mod hashing;
mod index;
mod located;
mod metrics;
mod reads;
mod row_access_routes;
mod security;
mod summaries;
mod trees;
mod usability;
mod warnings;

use crate::nodes::{member, text};
use crate::outline::{ModelOutline, outline};
use crate::pointer::JsonPointer;
use assembly::Assembly;
use checks::{CheckedModel, recorded_checks};
use datasets::by_name;
use located::{declared, descriptions, location_of};

/// The outputs of E0.b to E0.g that E0.h reads; every substage succeeded (E0-3).
#[derive(Debug, Clone, Copy)]
pub(crate) struct ModelInput<'earlier, 'document> {
    /// E0.b: the selected semantic model and its compiled expressions.
    pub(crate) structure: &'earlier Structure<'document>,
    /// E0.c: the entries of ours and the declarations of each extension.
    pub(crate) extensions: &'earlier Extensions,
    /// E0.c: the access of every dataset, with `W0C-40` and `W0C-41`.
    pub(crate) access: &'earlier ResolvedAccess,
    /// E0.d: the elements a missing reference makes unusable.
    pub(crate) references: &'earlier References,
    /// E0.d: the derived metrics and their inputs.
    pub(crate) derived: &'earlier DerivedMetrics,
    /// E0.d: the context metrics and their bases.
    pub(crate) context: &'earlier ContextMetrics,
    /// E0.e: the relationship graph.
    pub(crate) relationships: &'earlier Relationships,
    /// E0.f: the types of the fields and metrics.
    pub(crate) typing: &'earlier Typing,
    /// E0.f: the types of the derived and context metrics.
    pub(crate) derived_typing: &'earlier Typing,
    /// E0.g: the classification of the metrics.
    pub(crate) classification: &'earlier Classification,
}

/// Builds the internal model of the semantic model `input.structure` selected in `document`.
pub(crate) fn build_internal_model(document: &Node, input: &ModelInput<'_, '_>) -> InternalModel {
    let selected = &input.structure.semantic_model;
    let model = outline(selected.node, &selected.pointer);
    let usability = usability::usability(input, &model);
    let assembly = Assembly::new(document, input, &model, usability);
    let datasets = by_name(
        model
            .datasets
            .iter()
            .map(|dataset| datasets::dataset(&assembly, dataset)),
        |dataset| &dataset.name,
    );
    let relationships = by_name(
        model
            .relationships
            .iter()
            .map(|relationship| graph::relationship(&assembly, relationship)),
        |relationship| &relationship.name,
    );
    let metrics = by_name(
        model
            .metrics
            .iter()
            .map(|metric| metrics::metric(&assembly, metric)),
        |metric| &metric.name,
    );
    let summaries = summaries_of(&assembly, &model, &datasets);
    let checked = CheckedModel {
        datasets: &datasets,
        relationships: &relationships,
        metrics: &metrics,
        summaries: &summaries,
    };
    let checks = recorded_checks(&checked);
    let parts = ModelParts {
        name: located::written(text(member(selected.node, "name")).unwrap_or("")),
        descriptions: descriptions(document, &selected.pointer),
        datasets,
        relationships,
        graph: graph::graph(&assembly),
        metrics,
        security_context: (input.extensions.security.context.as_ref())
            .map(security::security_context),
        summaries,
        checks,
        unusable: unusable_elements(&assembly, &model),
        warnings: warnings::warnings(document, input),
    };
    let hash = hashing::model_hash(&parts);
    InternalModel::new(parts, hash)
}

/// The summaries, ordered by summary dataset name, with their definition hashes.
fn summaries_of(
    assembly: &Assembly<'_, '_>,
    model: &ModelOutline<'_>,
    datasets: &[Dataset],
) -> Vec<Summary> {
    let mut summaries: Vec<Summary> = assembly
        .input
        .extensions
        .summaries
        .iter()
        .map(|declaration| {
            let name = model
                .datasets
                .iter()
                .find(|dataset| dataset.pointer == declaration.dataset)
                .map_or_else(|| declared(None), |dataset| declared(dataset.name.as_ref()));
            summaries::summary(declaration, name, datasets)
        })
        .collect();
    summaries.sort_by(|left, right| left.dataset.cmp(&right.dataset));
    summaries
}

/// Every unusable element, ordered by kind, then by qualified name (E0H-1.4).
fn unusable_elements(
    assembly: &Assembly<'_, '_>,
    model: &ModelOutline<'_>,
) -> Vec<UnusableElement> {
    let mut found: Vec<(ElementKind, Vec<NormalizedName>, &JsonPointer)> = Vec::new();
    for dataset in &model.datasets {
        let dataset_name = declared(dataset.name.as_ref());
        found.push((
            ElementKind::Dataset,
            vec![dataset_name.clone()],
            &dataset.pointer,
        ));
        for field in &dataset.fields {
            let name = vec![dataset_name.clone(), declared(field.name.as_ref())];
            found.push((ElementKind::Field, name, &field.pointer));
        }
        for metric in &dataset.metrics {
            let name = vec![dataset_name.clone(), declared(metric.name.as_ref())];
            found.push((ElementKind::Metric, name, &metric.pointer));
        }
    }
    for relationship in &model.relationships {
        let name = vec![declared(relationship.name.as_ref())];
        found.push((ElementKind::Relationship, name, &relationship.pointer));
    }
    for metric in &model.metrics {
        found.push((
            ElementKind::Metric,
            vec![declared(metric.name.as_ref())],
            &metric.pointer,
        ));
    }
    let usability = &assembly.usability;
    let mut unusable: Vec<UnusableElement> = found
        .into_iter()
        .filter(|(_, _, pointer)| !usability.is_usable(pointer))
        .map(|(kind, name, pointer)| UnusableElement {
            kind,
            name,
            load_warning: usability.load_warning(pointer),
            query_errors: usability.codes(pointer),
            row_access_cut: usability.row_access_cut(pointer),
            location: location_of(assembly.document, pointer),
        })
        .collect();
    unusable.sort_by(|left, right| (left.kind, &left.name).cmp(&(right.kind, &right.name)));
    unusable
}

#[cfg(test)]
mod tests;

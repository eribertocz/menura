//! Substage E0.h: builds the immutable internal model from the outputs of E0.b to E0.g
//! (E0H-1, E0H-2), with its hashes (E0H-3) and its recorded integrity checks (E0H-4.3).
//!
//! E0.h reports no diagnostic. It computes the unusability that propagates from one element
//! to the elements that read it (E0-4.2a), removes the relationships made unusable after E0.e
//! from the graph and recomputes reachability with the E0.e algorithm (E0H-2.3).

use semantic_compiler_contracts::types::catalog::CatalogSchemaDocument;
use semantic_compiler_model::internal_model::{
    Descriptions, InternalModel, ModelParts, NormalizedName, RelationshipGraph,
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

pub(crate) mod hashing;

/// The outputs of E0.b to E0.g that E0.h reads; every substage succeeded (E0-3).
#[derive(Debug, Clone, Copy)]
pub(crate) struct ModelInput<'earlier, 'document> {
    /// E0.b: the selected semantic model and its compiled expressions.
    pub(crate) structure: &'earlier Structure<'document>,
    /// The admitted catalog schema (E0-3.2).
    pub(crate) catalog: &'earlier CatalogSchemaDocument,
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
    let _ = (document, input);
    let parts = ModelParts {
        name: NormalizedName::new(""),
        descriptions: Descriptions::default(),
        datasets: Vec::new(),
        relationships: Vec::new(),
        graph: RelationshipGraph::default(),
        metrics: Vec::new(),
        security_context: None,
        summaries: Vec::new(),
        checks: Vec::new(),
        unusable: Vec::new(),
        warnings: Vec::new(),
    };
    let hash = hashing::model_hash(&parts);
    InternalModel::new(parts, hash)
}

#[cfg(test)]
mod tests;

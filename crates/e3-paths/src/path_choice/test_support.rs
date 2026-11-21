//! Relationship graphs and bound queries shared by the unit tests of path choice.

use menura_contracts::Sha256Hash;
use menura_model::internal_model::{
    Dataset, Descriptions, GraphEdge, InternalModel, Metric, ModelParts, RelationshipGraph,
};
use menura_representations::bound_query::{AuthorizedSubgraph, BoundPath, BoundQueryParts};
use menura_representations::paths::JoinStep;

use crate::test_support::{at, name};

/// The edge `relationship` from `from` to `to`.
pub(crate) fn edge(relationship: &str, from: &str, to: &str) -> GraphEdge {
    GraphEdge {
        relationship: name(relationship),
        from: name(from),
        to: name(to),
        one_to_one: false,
    }
}

/// The step that crosses `relationship` from `from` to `to`.
pub(crate) fn step(relationship: &str, from: &str, to: &str) -> JoinStep {
    JoinStep {
        relationship: name(relationship),
        from: name(from),
        to: name(to),
    }
}

/// The model with `datasets`, the semantic-model `metrics` and a graph of `edges`, which it
/// orders by name as the internal model does (E0H-2.3).
pub(crate) fn graph_model(
    datasets: Vec<Dataset>,
    mut metrics: Vec<Metric>,
    mut edges: Vec<GraphEdge>,
) -> InternalModel {
    metrics.sort_by(|left, right| left.name.cmp(&right.name));
    edges.sort_by(|left, right| left.relationship.cmp(&right.relationship));
    let parts = ModelParts {
        name: name("SALES"),
        descriptions: Descriptions::default(),
        datasets,
        relationships: Vec::new(),
        graph: RelationshipGraph {
            edges,
            reachability: Vec::new(),
            several_paths: Vec::new(),
        },
        metrics,
        security_context: None,
        summaries: Vec::new(),
        checks: Vec::new(),
        unusable: Vec::new(),
        warnings: Vec::new(),
    };
    let hash = Sha256Hash::try_from("0".repeat(64)).expect("a valid hash");
    InternalModel::new(parts, hash)
}

/// The subgraph that authorizes every dataset and every relationship of `model`.
pub(crate) fn whole_subgraph(model: &InternalModel) -> AuthorizedSubgraph {
    let graph = &model.parts().graph;
    let mut datasets = Vec::new();
    for edge in &graph.edges {
        datasets.push(edge.from.clone());
        datasets.push(edge.to.clone());
    }
    let relationships = graph
        .edges
        .iter()
        .map(|edge| edge.relationship.clone())
        .collect();
    AuthorizedSubgraph::new(datasets, relationships)
}

/// The `paths` entry at `/query/paths/<index>` from `from` to `to` through `relationships`.
pub(crate) fn chosen(from: &str, to: &str, relationships: &[&str], index: usize) -> BoundPath {
    BoundPath::new(
        name(from),
        name(to),
        relationships
            .iter()
            .map(|relationship| name(relationship))
            .collect(),
        at(&format!("/query/paths/{index}")),
    )
    .expect("at least one relationship")
}

/// The query parts with `paths` and the subgraph that authorizes all of `model`.
pub(crate) fn parts_with_paths(model: &InternalModel, paths: Vec<BoundPath>) -> BoundQueryParts {
    BoundQueryParts {
        paths,
        subgraph: whole_subgraph(model),
        ..BoundQueryParts::default()
    }
}

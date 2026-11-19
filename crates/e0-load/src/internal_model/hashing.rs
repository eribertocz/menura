//! The model hash (E0H-3.2): the CON-2 hash of the whole internal model except source
//! locations and warnings, with each unusable element and its query error codes.

use semantic_compiler_contracts::{Sha256Hash, hash_value};
use semantic_compiler_model::internal_model::{
    DatasetPair, ModelParts, PathCount, RecordedCheck, RelationshipGraph, Summary, UnusableElement,
};
use serde_json::Value as Json;

use super::documents::{
    codes, dataset, descriptions, metric, name, names, object, relationship, security_context,
    text, tree,
};

/// The model hash of `parts` (E0H-3.2, MDL-3.1).
pub(crate) fn model_hash(parts: &ModelParts) -> Sha256Hash {
    hash_value(&document(parts))
}

fn document(parts: &ModelParts) -> Json {
    object([
        ("name", name(&parts.name)),
        ("descriptions", descriptions(&parts.descriptions)),
        (
            "datasets",
            Json::Array(parts.datasets.iter().map(dataset).collect()),
        ),
        (
            "relationships",
            Json::Array(parts.relationships.iter().map(relationship).collect()),
        ),
        ("graph", graph(&parts.graph)),
        (
            "metrics",
            Json::Array(parts.metrics.iter().map(metric).collect()),
        ),
        (
            "security_context",
            parts
                .security_context
                .as_ref()
                .map_or(Json::Null, security_context),
        ),
        (
            "summaries",
            Json::Array(parts.summaries.iter().map(summary).collect()),
        ),
        (
            "checks",
            Json::Array(parts.checks.iter().map(check).collect()),
        ),
        (
            "unusable",
            Json::Array(parts.unusable.iter().map(unusable).collect()),
        ),
    ])
}

fn graph(graph: &RelationshipGraph) -> Json {
    let edges = graph
        .edges
        .iter()
        .map(|edge| {
            object([
                ("relationship", name(&edge.relationship)),
                ("from", name(&edge.from)),
                ("to", name(&edge.to)),
                ("one_to_one", Json::Bool(edge.one_to_one)),
            ])
        })
        .collect();
    let reachability = graph
        .reachability
        .iter()
        .map(|reach| {
            let reached = reach
                .reached
                .iter()
                .map(|target| {
                    object([
                        ("dataset", name(&target.dataset)),
                        (
                            "paths",
                            text(match target.paths {
                                PathCount::One => "one",
                                PathCount::Several => "several",
                            }),
                        ),
                    ])
                })
                .collect();
            object([
                ("dataset", name(&reach.dataset)),
                ("reached", Json::Array(reached)),
            ])
        })
        .collect();
    object([
        ("edges", Json::Array(edges)),
        ("reachability", Json::Array(reachability)),
        (
            "several_paths",
            Json::Array(graph.several_paths.iter().map(pair).collect()),
        ),
    ])
}

fn pair(pair: &DatasetPair) -> Json {
    object([("from", name(&pair.from)), ("to", name(&pair.to))])
}

fn summary(summary: &Summary) -> Json {
    let measures = summary
        .measures
        .iter()
        .map(|measure| {
            object([
                ("field", name(&measure.field)),
                ("base_aggregate", tree(measure.base_aggregate.as_ref())),
            ])
        })
        .collect();
    object([
        ("dataset", name(&summary.dataset)),
        ("of", name(&summary.of)),
        ("measures", Json::Array(measures)),
        ("definition_hash", text(summary.definition_hash.as_str())),
    ])
}

fn check(check: &RecordedCheck) -> Json {
    object([
        ("check_id", text(&check.check_id)),
        ("definition_hash", text(check.definition_hash.as_str())),
    ])
}

fn unusable(element: &UnusableElement) -> Json {
    object([
        ("kind", text(element.kind.as_str())),
        ("name", names(&element.name)),
        ("query_errors", codes(&element.query_errors)),
    ])
}

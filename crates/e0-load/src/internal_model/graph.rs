//! The relationships and the graph of the usable ones (E0H-2 "Graph", E0H-2.3).
//!
//! The graph keeps the nodes of E0.e and drops every edge an element made unusable after E0.e
//! (`E0F-05`, E0-4.2a); reachability and the multiple-path pairs are then recomputed over the
//! remaining edges with the E0.e algorithm.

use menura_model::internal_model::{
    DatasetPair, DatasetReach, GraphEdge, NormalizedName, PathCount, ReachedDataset, Relationship,
    RelationshipGraph, TemporalJoin,
};

use super::assembly::Assembly;
use super::datasets::by_name;
use super::located::{declared, descriptions, location_of, normalized, written};
use crate::outline::RelationshipOutline;
use crate::relationships::paths::reachability;
use crate::relationships::{self, PathCount as Paths};

/// The relationship `outline`, usable or not.
pub(super) fn relationship(
    assembly: &Assembly<'_, '_>,
    outline: &RelationshipOutline<'_>,
) -> Relationship {
    let names = |columns: &[crate::outline::DeclaredName<'_>]| {
        columns
            .iter()
            .map(|column| normalized(&column.name))
            .collect()
    };
    Relationship {
        name: declared(outline.name.as_ref()),
        from: declared(outline.from.as_ref()),
        to: declared(outline.to.as_ref()),
        from_columns: names(&outline.from_columns),
        to_columns: names(&outline.to_columns),
        temporal: assembly
            .input
            .extensions
            .temporal_declaration(&outline.pointer)
            .map(|declaration| TemporalJoin {
                from_time: written(&declaration.from_time),
                population: written(&declaration.population),
            }),
        descriptions: descriptions(assembly.document, &outline.pointer),
        query_errors: assembly.usability.codes(&outline.pointer),
        location: location_of(assembly.document, &outline.pointer),
    }
}

/// The graph of the relationships that are still usable (E0H-2.3).
pub(super) fn graph(assembly: &Assembly<'_, '_>) -> RelationshipGraph {
    let remaining = remaining_graph(assembly);
    let (reachable, several_paths) = reachability(&remaining);
    let node = |index: usize| -> NormalizedName {
        remaining
            .datasets
            .get(index)
            .map_or_else(|| declared(None), |dataset| normalized(&dataset.name))
    };
    let edges = remaining.edges.iter().map(|edge| GraphEdge {
        relationship: normalized(&edge.name),
        from: node(edge.from.0),
        to: node(edge.to.0),
        one_to_one: edge.one_to_one,
    });
    let reach = reachable
        .iter()
        .enumerate()
        .map(|(index, reached)| DatasetReach {
            dataset: node(index),
            reached: by_name(
                reached.iter().map(|target| ReachedDataset {
                    dataset: node(target.dataset.0),
                    paths: match target.paths {
                        Paths::One => PathCount::One,
                        Paths::Several => PathCount::Several,
                    },
                }),
                |target| &target.dataset,
            ),
        });
    let mut pairs: Vec<DatasetPair> = several_paths
        .iter()
        .map(|pair| DatasetPair {
            from: node(pair.from.0),
            to: node(pair.to.0),
        })
        .collect();
    pairs.sort();
    RelationshipGraph {
        edges: by_name(edges, |edge| &edge.relationship),
        reachability: by_name(reach, |reach| &reach.dataset),
        several_paths: pairs,
    }
}

/// The E0.e graph without the edges made unusable after E0.e.
fn remaining_graph(assembly: &Assembly<'_, '_>) -> relationships::RelationshipGraph {
    let analyzed = &assembly.input.relationships.graph;
    relationships::RelationshipGraph {
        datasets: analyzed.datasets.clone(),
        edges: analyzed
            .edges
            .iter()
            .filter(|edge| assembly.usability.is_usable(&edge.pointer))
            .cloned()
            .collect(),
    }
}

//! Builds the relationship graph (E0E-2) from the datasets and the checked relationships.

use std::collections::{HashMap, HashSet};

use super::declarations::{DatasetDeclaration, RelationshipDeclaration};
use super::output::{DatasetIndex, Edge, GraphDataset, RelationshipGraph};
use super::validation::Verdict;
use crate::pointer::JsonPointer;

/// The datasets that are nodes of the graph: every dataset except `summaries`, in document
/// order (E0E-2.1). The position of a dataset is its [`DatasetIndex`].
pub(crate) fn nodes<'model, 'document>(
    datasets: &'model [DatasetDeclaration<'document>],
    summaries: &HashSet<&JsonPointer>,
) -> Vec<&'model DatasetDeclaration<'document>> {
    datasets
        .iter()
        .filter(|dataset| !summaries.contains(&dataset.place.pointer))
        .collect()
}

/// The graph whose nodes are `nodes` (see [`nodes`]) and whose edges are the relationships
/// judged [`Verdict::Edge`] (E0E-2.2).
pub(crate) fn build(
    nodes: &[&DatasetDeclaration<'_>],
    judged: &[(&RelationshipDeclaration<'_>, Verdict<'_, '_>)],
) -> RelationshipGraph {
    let index: HashMap<&JsonPointer, DatasetIndex> = nodes
        .iter()
        .enumerate()
        .map(|(position, dataset)| (&dataset.place.pointer, DatasetIndex(position)))
        .collect();
    let edges = judged
        .iter()
        .filter_map(|(relationship, verdict)| {
            let Verdict::Edge {
                from,
                to,
                one_to_one,
            } = verdict
            else {
                return None;
            };
            Some(Edge {
                name: relationship.name.name.clone(),
                pointer: relationship.place.pointer.clone(),
                from: *index.get(&from.place.pointer)?,
                to: *index.get(&to.place.pointer)?,
                one_to_one: *one_to_one,
            })
        })
        .collect();
    RelationshipGraph {
        datasets: nodes
            .iter()
            .map(|dataset| GraphDataset {
                name: dataset.name.name.clone(),
                pointer: dataset.place.pointer.clone(),
            })
            .collect(),
        edges,
    }
}

//! The chains of relationships that the visible rows of a dataset follow (E4-6.5, E4-6.5b,
//! E4-6.6): from a dataset, every usable relationship to a dataset that has row policies or
//! reaches one, recursively, except a relationship back to a dataset already on the chain.

use std::collections::BTreeSet;

use menura_model::internal_model::{GraphEdge, InternalModel, NormalizedName};
use menura_representations::paths::JoinStep;

/// Every chain E4-6.5 follows from `start`, ordered by length, then by the relationship names
/// along it. A chain is a path of one or more steps from `start`; each of its prefixes is a
/// chain too.
pub(crate) fn propagation_chains(
    model: &InternalModel,
    start: &NormalizedName,
) -> Vec<Vec<JoinStep>> {
    let edges = &model.parts().graph.edges;
    let mut chains = Vec::new();
    let mut on_chain = vec![start.clone()];
    let mut current = Vec::new();
    extend(model, edges, &mut on_chain, &mut current, &mut chains);
    chains.sort_by(|left, right| {
        left.len()
            .cmp(&right.len())
            .then_with(|| relationship_names(left).cmp(&relationship_names(right)))
    });
    chains
}

/// Adds every chain that extends `current`, whose datasets are `on_chain`, to `chains`.
fn extend(
    model: &InternalModel,
    edges: &[GraphEdge],
    on_chain: &mut Vec<NormalizedName>,
    current: &mut Vec<JoinStep>,
    chains: &mut Vec<Vec<JoinStep>>,
) {
    let Some(last) = on_chain.last().cloned() else {
        return;
    };
    for edge in edges.iter().filter(|edge| edge.from == last) {
        if on_chain.contains(&edge.to) || !governed(model, edges, &edge.to) {
            continue;
        }
        current.push(JoinStep {
            relationship: edge.relationship.clone(),
            from: edge.from.clone(),
            to: edge.to.clone(),
        });
        chains.push(current.clone());
        on_chain.push(edge.to.clone());
        extend(model, edges, on_chain, current, chains);
        on_chain.pop();
        current.pop();
    }
}

/// Whether `dataset` has row policies or reaches, along usable relationships, a dataset that
/// has some (E4-6.5).
fn governed(model: &InternalModel, edges: &[GraphEdge], dataset: &NormalizedName) -> bool {
    let mut seen: BTreeSet<&NormalizedName> = BTreeSet::from([dataset]);
    let mut pending = vec![dataset];
    while let Some(current) = pending.pop() {
        if has_policies(model, current) {
            return true;
        }
        for edge in edges.iter().filter(|edge| &edge.from == current) {
            if seen.insert(&edge.to) {
                pending.push(&edge.to);
            }
        }
    }
    false
}

/// Whether `dataset` declares row policies.
pub(crate) fn has_policies(model: &InternalModel, dataset: &NormalizedName) -> bool {
    model
        .dataset(dataset)
        .is_some_and(|found| !found.row_policies.is_empty())
}

fn relationship_names(chain: &[JoinStep]) -> Vec<&NormalizedName> {
    chain.iter().map(|step| &step.relationship).collect()
}

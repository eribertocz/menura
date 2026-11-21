//! The relationships that the visible rows of a dataset follow (E4-6.5, E4-6.5b, E4-6.6): from
//! a dataset, every usable relationship to a dataset that has row policies or reaches one,
//! except a relationship back to a dataset already on the current chain.

use std::collections::BTreeSet;

use menura_model::internal_model::{GraphEdge, InternalModel, NormalizedName};

/// The relationships E4-6.5 follows from `dataset`, the last dataset of `chain`, ordered by
/// relationship name. Each one is followed separately, even when several reach the same
/// dataset (E4-6.6).
pub(crate) fn followed_relationships<'model>(
    model: &'model InternalModel,
    dataset: &NormalizedName,
    chain: &[NormalizedName],
) -> Vec<&'model GraphEdge> {
    let edges = &model.parts().graph.edges;
    let mut followed: Vec<&GraphEdge> = edges
        .iter()
        .filter(|edge| &edge.from == dataset)
        .filter(|edge| !chain.contains(&edge.to))
        .filter(|edge| governed(model, edges, &edge.to))
        .collect();
    followed.sort_by(|left, right| left.relationship.cmp(&right.relationship));
    followed
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

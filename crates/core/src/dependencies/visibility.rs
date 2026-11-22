//! The visibility inputs of each dataset (DEP-3.4): what V(D) of E4-6.5 reads, for every user.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use menura_model::internal_model::{GraphEdge, InternalModel, NormalizedName};

/// The usable relationship graph, indexed by dataset, with the datasets that have row
/// policies or reach one (E4-6.5).
pub(super) struct PolicyGraph<'model> {
    /// The relationships that leave each dataset, ordered by relationship name.
    outgoing: BTreeMap<&'model NormalizedName, Vec<&'model GraphEdge>>,
    /// The datasets that have row policies or reach, along usable relationships, one that has.
    governed: BTreeSet<&'model NormalizedName>,
}

impl<'model> PolicyGraph<'model> {
    /// The index of the usable graph of `model`.
    pub(super) fn new(model: &'model InternalModel) -> Self {
        let mut outgoing: BTreeMap<&NormalizedName, Vec<&GraphEdge>> = BTreeMap::new();
        let mut incoming: BTreeMap<&NormalizedName, Vec<&NormalizedName>> = BTreeMap::new();
        for edge in &model.parts().graph.edges {
            outgoing.entry(&edge.from).or_default().push(edge);
            incoming.entry(&edge.to).or_default().push(&edge.from);
        }
        for edges in outgoing.values_mut() {
            edges.sort_by(|left, right| left.relationship.cmp(&right.relationship));
        }
        let mut governed: BTreeSet<&NormalizedName> = model
            .parts()
            .datasets
            .iter()
            .filter(|dataset| !dataset.row_policies.is_empty())
            .map(|dataset| &dataset.name)
            .collect();
        let mut pending: VecDeque<&NormalizedName> = governed.iter().copied().collect();
        while let Some(current) = pending.pop_front() {
            for source in incoming.get(current).into_iter().flatten() {
                if governed.insert(source) {
                    pending.push_back(source);
                }
            }
        }
        Self { outgoing, governed }
    }

    /// The relationships V(D) follows from `dataset`, the last dataset of `chain`: every usable
    /// one to a dataset that has row policies or reaches one, except one back to a dataset of
    /// the chain (E4-6.5, E4-6.5b, E4-6.6).
    pub(super) fn followed(
        &self,
        dataset: &NormalizedName,
        chain: &[&NormalizedName],
    ) -> Vec<&'model GraphEdge> {
        self.outgoing
            .get(dataset)
            .into_iter()
            .flatten()
            .copied()
            .filter(|edge| !chain.contains(&&edge.to) && self.governed.contains(&edge.to))
            .collect()
    }
}

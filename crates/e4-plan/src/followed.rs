//! The relationships that the visible rows of a dataset follow (E4-6.5, E4-6.5b, E4-6.6): from
//! a dataset, every usable relationship to a dataset that has row policies or reaches one,
//! except a relationship back to a dataset already on the current chain. The usable graph is
//! indexed once per plan.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use menura_model::internal_model::{GraphEdge, InternalModel, NormalizedName};

/// The usable relationship graph, indexed by dataset, with the datasets that have row
/// policies or reach one.
#[derive(Debug)]
pub(crate) struct PolicyGraph<'model> {
    /// The relationships that leave each dataset, ordered by relationship name.
    outgoing: BTreeMap<&'model NormalizedName, Vec<&'model GraphEdge>>,
    /// The datasets that have row policies or reach, along usable relationships, a dataset
    /// that has some.
    governed: BTreeSet<&'model NormalizedName>,
}

impl<'model> PolicyGraph<'model> {
    /// The index of the usable graph of `model`.
    pub(crate) fn new(model: &'model InternalModel) -> Self {
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

    /// The relationships E4-6.5 follows from `dataset`, the last dataset of `chain`, ordered
    /// by relationship name. Each one is followed separately, even when several reach the same
    /// dataset (E4-6.6).
    pub(crate) fn followed(
        &self,
        dataset: &NormalizedName,
        chain: &[NormalizedName],
    ) -> Vec<&'model GraphEdge> {
        self.leaving(dataset)
            .filter(|edge| !chain.contains(&edge.to) && self.governed.contains(&edge.to))
            .collect()
    }

    /// The length of the shortest path from `home` to every dataset it reaches, `home`
    /// included at 0.
    pub(crate) fn distances(
        &self,
        home: &'model NormalizedName,
    ) -> Vec<(&'model NormalizedName, usize)> {
        let mut seen: BTreeSet<&NormalizedName> = BTreeSet::from([home]);
        let mut found = Vec::new();
        let mut pending = VecDeque::from([(home, 0)]);
        while let Some((current, distance)) = pending.pop_front() {
            found.push((current, distance));
            for edge in self.leaving(current) {
                if seen.insert(&edge.to) {
                    pending.push_back((&edge.to, distance + 1));
                }
            }
        }
        found
    }

    /// The temporal relationships that the visible rows of `home` follow on their way to
    /// `target` (E4-6.5, VAL-2.17), as `followed` walks them from `home`, ordered by name.
    pub(crate) fn temporal_towards(
        &self,
        home: &'model NormalizedName,
        target: &NormalizedName,
        model: &InternalModel,
    ) -> BTreeSet<&'model NormalizedName> {
        let mut found = BTreeSet::new();
        let mut pending: Vec<(Vec<NormalizedName>, Vec<&'model NormalizedName>)> =
            vec![(vec![home.clone()], Vec::new())];
        while let Some((chain, temporal)) = pending.pop() {
            let Some(current) = chain.last() else {
                continue;
            };
            for edge in self.followed(current, &chain) {
                let mut next_temporal = temporal.clone();
                let is_temporal = model
                    .relationship(&edge.relationship)
                    .is_some_and(|relationship| relationship.temporal.is_some());
                if is_temporal {
                    next_temporal.push(&edge.relationship);
                }
                if &edge.to == target {
                    found.extend(next_temporal.iter().copied());
                }
                let mut next_chain = chain.clone();
                next_chain.push(edge.to.clone());
                pending.push((next_chain, next_temporal));
            }
        }
        found
    }

    fn leaving(&self, dataset: &NormalizedName) -> impl Iterator<Item = &'model GraphEdge> {
        self.outgoing.get(dataset).into_iter().flatten().copied()
    }
}

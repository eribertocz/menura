//! Reachability over the authorized subgraph through many-to-one steps (E3-3.1, E2-4.8).

use std::collections::{BTreeMap, BTreeSet};

use menura_model::internal_model::{NormalizedName, RelationshipGraph};
use menura_representations::bound_query::AuthorizedSubgraph;

/// The many-to-one steps the query may take: the edges of the relationship graph whose
/// relationship is authorized (E2-4.8). The graph has no self-relationship (E0E-3.5).
#[derive(Debug, Default)]
pub(crate) struct AuthorizedGraph<'model> {
    /// For each dataset, the datasets one step reaches from it.
    next: BTreeMap<&'model NormalizedName, Vec<&'model NormalizedName>>,
}

impl<'model> AuthorizedGraph<'model> {
    /// The edges of `graph` that `subgraph` authorizes.
    pub(crate) fn new(graph: &'model RelationshipGraph, subgraph: &AuthorizedSubgraph) -> Self {
        let mut next: BTreeMap<&NormalizedName, Vec<&NormalizedName>> = BTreeMap::new();
        for edge in &graph.edges {
            if subgraph.contains_relationship(&edge.relationship) {
                next.entry(&edge.from).or_default().push(&edge.to);
            }
        }
        Self { next }
    }

    /// Whether a path leads from `from` to `to`. The path from a dataset to itself is empty,
    /// so a dataset always reaches itself (E3-3.1).
    pub(crate) fn reaches(&self, from: &NormalizedName, to: &NormalizedName) -> bool {
        if from == to {
            return true;
        }
        let mut visited: BTreeSet<&NormalizedName> = BTreeSet::from([from]);
        let mut pending: Vec<&NormalizedName> = vec![from];
        while let Some(dataset) = pending.pop() {
            for reached in self.next.get(dataset).into_iter().flatten() {
                if *reached == to {
                    return true;
                }
                if visited.insert(reached) {
                    pending.push(reached);
                }
            }
        }
        false
    }

    /// Whether every dataset of `homes` reaches `to`: the reach of a metric whose components'
    /// home datasets are `homes` (E3-3.4).
    pub(crate) fn all_reach(&self, homes: &[&NormalizedName], to: &NormalizedName) -> bool {
        homes.iter().all(|home| self.reaches(home, to))
    }
}

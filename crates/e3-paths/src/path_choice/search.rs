//! The paths between two datasets in the authorized subgraph (E2-4.8, E3-3.1).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use menura_model::internal_model::{GraphEdge, NormalizedName, RelationshipGraph};
use menura_representations::bound_query::AuthorizedSubgraph;
use menura_representations::paths::JoinStep;

/// How many paths lead from one dataset to another, with the path when there is one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PathsFound {
    /// No path.
    None,
    /// Exactly one path, with its steps in order.
    One(Vec<JoinStep>),
    /// Two or more paths.
    Several,
}

/// The edges of the relationship graph that the authorized subgraph keeps, indexed by
/// relationship name and by the dataset they leave.
#[derive(Debug)]
pub(crate) struct AuthorizedEdges<'model> {
    by_relationship: BTreeMap<&'model NormalizedName, &'model GraphEdge>,
    leaving: BTreeMap<&'model NormalizedName, Vec<&'model GraphEdge>>,
}

impl<'model> AuthorizedEdges<'model> {
    /// The edges of `graph` whose relationship `subgraph` authorizes. The graph holds no
    /// self-relationship (E0E-3.5), and its edges are ordered by relationship name, so the
    /// edges leaving each dataset keep that order.
    pub(crate) fn new(graph: &'model RelationshipGraph, subgraph: &AuthorizedSubgraph) -> Self {
        let mut by_relationship = BTreeMap::new();
        let mut leaving: BTreeMap<&NormalizedName, Vec<&GraphEdge>> = BTreeMap::new();
        for edge in &graph.edges {
            if subgraph.contains_relationship(&edge.relationship) {
                by_relationship.insert(&edge.relationship, edge);
                leaving.entry(&edge.from).or_default().push(edge);
            }
        }
        Self {
            by_relationship,
            leaving,
        }
    }

    /// The authorized edge of `relationship`, when there is one.
    pub(crate) fn edge(&self, relationship: &NormalizedName) -> Option<&'model GraphEdge> {
        self.by_relationship.get(relationship).copied()
    }

    /// The paths from `from` to `to`, a different dataset: each step crosses an edge in its
    /// many-to-one direction, and no path repeats a dataset (E3-3.1).
    ///
    /// The search takes the shortest path, then looks for a second one as Yen's algorithm
    /// does with `k = 2`: a second simple path leaves the first at some dataset of it, so
    /// for each dataset of the first path it searches a path to `to` that avoids the datasets
    /// before it and the edge the first path takes there. With `d` datasets and `e` edges
    /// that is at most `d + 1` breadth-first searches, `O(d · (d + e) · log d)`, whatever
    /// cycles the graph has.
    pub(crate) fn paths(&self, from: &NormalizedName, to: &NormalizedName) -> PathsFound {
        let Some(first) = self.shortest(from, to, &BTreeSet::new(), None) else {
            return PathsFound::None;
        };
        let mut before: BTreeSet<&NormalizedName> = BTreeSet::new();
        for edge in &first {
            if self
                .shortest(&edge.from, to, &before, Some(&edge.relationship))
                .is_some()
            {
                return PathsFound::Several;
            }
            before.insert(&edge.from);
        }
        PathsFound::One(first.iter().map(|edge| step(edge)).collect())
    }

    /// A shortest path from `from` to `to` that enters no dataset of `avoided` and crosses no
    /// edge named `avoided_edge`, taking the edges of each dataset in relationship order.
    fn shortest(
        &self,
        from: &NormalizedName,
        to: &NormalizedName,
        avoided: &BTreeSet<&NormalizedName>,
        avoided_edge: Option<&NormalizedName>,
    ) -> Option<Vec<&'model GraphEdge>> {
        let mut arrived_by: BTreeMap<&NormalizedName, &'model GraphEdge> = BTreeMap::new();
        let mut pending = VecDeque::from([from]);
        while let Some(dataset) = pending.pop_front() {
            for edge in self.leaving.get(dataset).into_iter().flatten() {
                let next = &edge.to;
                let taken = next == from || arrived_by.contains_key(next);
                if taken || avoided.contains(next) || avoided_edge == Some(&edge.relationship) {
                    continue;
                }
                arrived_by.insert(next, edge);
                if next == to {
                    return Some(walk_back(&arrived_by, from, to));
                }
                pending.push_back(next);
            }
        }
        None
    }
}

/// The edges from `from` to `to` that `arrived_by` records, in path order.
fn walk_back<'model>(
    arrived_by: &BTreeMap<&NormalizedName, &'model GraphEdge>,
    from: &NormalizedName,
    to: &NormalizedName,
) -> Vec<&'model GraphEdge> {
    let mut edges = Vec::new();
    let mut current = to;
    while current != from {
        let Some(edge) = arrived_by.get(current) else {
            break;
        };
        edges.push(*edge);
        current = &edge.from;
    }
    edges.reverse();
    edges
}

/// The join step that crosses `edge`.
pub(crate) fn step(edge: &GraphEdge) -> JoinStep {
    JoinStep {
        relationship: edge.relationship.clone(),
        from: edge.from.clone(),
        to: edge.to.clone(),
    }
}

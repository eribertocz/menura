//! The paths between two datasets in the authorized subgraph (E2-4.8, E3-3.1).

use std::collections::BTreeSet;

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

/// The edges of the relationship graph that the authorized subgraph keeps, ordered by
/// relationship name.
#[derive(Debug)]
pub(crate) struct AuthorizedEdges<'model> {
    edges: Vec<&'model GraphEdge>,
}

impl<'model> AuthorizedEdges<'model> {
    /// The edges of `graph` whose relationship `subgraph` authorizes. The graph holds no
    /// self-relationship (E0E-3.5).
    pub(crate) fn new(graph: &'model RelationshipGraph, subgraph: &AuthorizedSubgraph) -> Self {
        let edges = graph
            .edges
            .iter()
            .filter(|edge| subgraph.contains_relationship(&edge.relationship))
            .collect();
        Self { edges }
    }

    /// The authorized edge of `relationship`, when there is one.
    pub(crate) fn edge(&self, relationship: &NormalizedName) -> Option<&'model GraphEdge> {
        self.edges
            .iter()
            .find(|edge| &edge.relationship == relationship)
            .copied()
    }

    /// The paths from `from` to `to`, a different dataset: each step crosses an edge in its
    /// many-to-one direction, and no path repeats a dataset (E3-3.1). The search stops at the
    /// second path, and enters only datasets from which `to` can be reached.
    pub(crate) fn paths(&self, from: &NormalizedName, to: &NormalizedName) -> PathsFound {
        let mut search = Search {
            edges: &self.edges,
            to,
            reaching: self.datasets_reaching(to),
            visited: vec![from],
            steps: Vec::new(),
            found: Vec::new(),
        };
        search.walk(from);
        let mut found = search.found;
        match (found.pop(), found.is_empty()) {
            (None, _) => PathsFound::None,
            (Some(steps), true) => PathsFound::One(steps),
            (Some(_), false) => PathsFound::Several,
        }
    }

    /// The datasets from which `to` can be reached, `to` included.
    fn datasets_reaching<'name>(
        &'name self,
        to: &'name NormalizedName,
    ) -> BTreeSet<&'name NormalizedName> {
        let mut reaching = BTreeSet::from([to]);
        let mut pending = vec![to];
        while let Some(reached) = pending.pop() {
            for edge in self.edges.iter().filter(|edge| &edge.to == reached) {
                if reaching.insert(&edge.from) {
                    pending.push(&edge.from);
                }
            }
        }
        reaching
    }
}

/// A depth-first walk that collects at most two paths.
struct Search<'walk> {
    edges: &'walk [&'walk GraphEdge],
    to: &'walk NormalizedName,
    reaching: BTreeSet<&'walk NormalizedName>,
    visited: Vec<&'walk NormalizedName>,
    steps: Vec<JoinStep>,
    found: Vec<Vec<JoinStep>>,
}

impl Search<'_> {
    const ENOUGH: usize = 2;

    fn walk(&mut self, current: &NormalizedName) {
        if current == self.to {
            self.found.push(self.steps.clone());
            return;
        }
        let edges = self.edges;
        for edge in edges.iter().filter(|edge| &edge.from == current) {
            if self.found.len() >= Self::ENOUGH {
                return;
            }
            if !self.reaching.contains(&edge.to) || self.visited.contains(&&edge.to) {
                continue;
            }
            self.visited.push(&edge.to);
            self.steps.push(JoinStep {
                relationship: edge.relationship.clone(),
                from: edge.from.clone(),
                to: edge.to.clone(),
            });
            self.walk(&edge.to);
            self.steps.pop();
            self.visited.pop();
        }
    }
}

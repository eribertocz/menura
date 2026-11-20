//! The branches: the output of E3 (E3-4, E3-5), each with its home dataset, components and
//! join tree.

mod join_tree;
#[cfg(test)]
mod tests;

pub use join_tree::{JoinStep, JoinTree};

use menura_model::internal_model::NormalizedName;

/// One component of a requested metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentKey {
    /// The position of the metric among the metrics of the bound query, in output order,
    /// from 0.
    pub metric: usize,
    /// The position of the component among the metric's components, from 0.
    pub component: usize,
}

/// The components evaluated over one home dataset, with the joins they need (E3-4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    /// The home dataset, or the anchor of a dimension-only query (E3-5.3).
    pub home: NormalizedName,
    /// The components whose home dataset it is; empty for a dimension-only query (E3-5.3).
    pub components: Vec<ComponentKey>,
    /// The union of the paths to every dimension and filter dataset (E3-4.2).
    pub join_tree: JoinTree,
}

/// The branches of a query, ordered by the normalized name of their home dataset (E3-4.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branches(Vec<Branch>);

impl Branches {
    /// The branches `branches`, ordered by home dataset (E3-4.3). Returns `None` when two of
    /// them share a home dataset: such components form one branch (E3-4.1).
    #[must_use]
    pub fn new(mut branches: Vec<Branch>) -> Option<Self> {
        branches.sort_by(|left, right| left.home.cmp(&right.home));
        let homes_differ = branches
            .windows(2)
            .all(|pair| matches!(pair, [left, right] if left.home != right.home));
        homes_differ.then_some(Self(branches))
    }

    /// The single branch of a dimension-only query: the anchor as home dataset and no
    /// components (E3-5.3).
    #[must_use]
    pub fn dimension_only(anchor: NormalizedName, join_tree: JoinTree) -> Self {
        Self(vec![Branch {
            home: anchor,
            components: Vec::new(),
            join_tree,
        }])
    }

    /// The branches, ordered by home dataset.
    #[must_use]
    pub fn as_slice(&self) -> &[Branch] {
        &self.0
    }
}

//! The branches: the output of E3 (E3-4, E3-5), each with its home dataset, components and
//! join tree of join instances.

mod element;
mod join_tree;
#[cfg(test)]
mod tests;

pub use element::{ElementPath, QueryElement};
pub use join_tree::{InstanceId, JoinInstance, JoinStep, JoinTree};

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

/// Which components a branch holds (E3-4.1). The order of the variants is the order of
/// branches with the same home dataset (E3-4.3).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BranchKind {
    /// The components of the metrics that are not semi-additive, or none for a
    /// dimension-only query.
    Plain,
    /// The components of one semi-additive metric.
    SemiAdditive {
        /// The normalized name of the metric.
        metric: NormalizedName,
    },
}

/// The components evaluated over one home dataset, with the joins they need (E3-4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    /// The home dataset, or the anchor of a dimension-only query (E3-5.3).
    pub home: NormalizedName,
    /// Which components it holds.
    pub kind: BranchKind,
    /// The components whose home dataset it is; empty for a dimension-only query (E3-5.3).
    pub components: Vec<ComponentKey>,
    /// The union of the paths to every dimension and filter dataset, as join instances, with
    /// the instance each query element is bound to (E3-4.2, E3-4.2a).
    pub join_tree: JoinTree,
}

/// The branches of a query, ordered by the normalized name of their home dataset, then, for
/// the same home dataset, the plain branch first and each semi-additive branch by the
/// normalized name of its metric (E3-4.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branches(Vec<Branch>);

impl Branches {
    /// The branches `branches`, in the order of E3-4.3. Returns `None` when two of them share
    /// a home dataset and a kind: such components form one branch (E3-4.1).
    #[must_use]
    pub fn new(mut branches: Vec<Branch>) -> Option<Self> {
        branches.sort_by(|left, right| (&left.home, &left.kind).cmp(&(&right.home, &right.kind)));
        let branches_differ = branches.windows(2).all(|pair| {
            matches!(pair, [left, right] if (&left.home, &left.kind) != (&right.home, &right.kind))
        });
        branches_differ.then_some(Self(branches))
    }

    /// The single branch of a dimension-only query: the anchor as home dataset and no
    /// components (E3-5.3).
    #[must_use]
    pub fn dimension_only(anchor: NormalizedName, join_tree: JoinTree) -> Self {
        Self(vec![Branch {
            home: anchor,
            kind: BranchKind::Plain,
            components: Vec::new(),
            join_tree,
        }])
    }

    /// The branches, in the order of E3-4.3.
    #[must_use]
    pub fn as_slice(&self) -> &[Branch] {
        &self.0
    }
}

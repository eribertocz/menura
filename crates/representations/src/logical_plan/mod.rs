//! The logical plan: the output of E4 and E5 (E4-3), a tree of typed nodes, each with its
//! output schema and origin.

mod items;
mod operation;
mod population;
mod schema;
mod snapshot;
#[cfg(test)]
mod snapshot_tests;
#[cfg(test)]
mod tests;

pub use items::{
    AggregateComponent, DerivedFormula, JoinCondition, JoinKind, NamedExpression, SortItem,
    StitchColumn,
};
pub use operation::Operation;
pub use population::{Membership, PopulationRule};
pub use schema::{OutputSchema, SchemaColumn};
pub use snapshot::{SnapshotMissing, SnapshotOver, SnapshotRule, TimeSnapshot, UnitKey};

use crate::origin::Origin;

/// One node of a logical plan (E4-3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanNode {
    operation: Operation,
    schema: OutputSchema,
    origin: Origin,
}

impl PlanNode {
    /// The node that performs `operation`, produces `schema` and comes from `origin`.
    #[must_use]
    pub fn new(operation: Operation, schema: OutputSchema, origin: Origin) -> Self {
        Self {
            operation,
            schema,
            origin,
        }
    }

    /// What the node does, with its inputs.
    #[must_use]
    pub fn operation(&self) -> &Operation {
        &self.operation
    }

    /// The columns the node produces.
    #[must_use]
    pub fn schema(&self) -> &OutputSchema {
        &self.schema
    }

    /// The input element the node comes from.
    #[must_use]
    pub fn origin(&self) -> &Origin {
        &self.origin
    }

    /// Whether the plan rooted at this node is a **simple plan**: it has no `Stitch` and no
    /// `Deduplicate` node (E4-4.2).
    #[must_use]
    pub fn is_simple_plan(&self) -> bool {
        !matches!(
            self.operation,
            Operation::Stitch { .. } | Operation::Deduplicate { .. }
        ) && self.inputs().into_iter().all(PlanNode::is_simple_plan)
    }

    /// The input nodes, in order: none for `Scan` and `Periods`, `left` then `right` for `Join`, `input`,
    /// `related_all` then `related_visible` for `SemiJoin`, `input` then `periods` for
    /// `Population`, `input` then the plans of its rule for `Snapshot` (`SnapshotRule::inputs`),
    /// the branches for `Stitch`, and the one input otherwise.
    #[must_use]
    pub fn inputs(&self) -> Vec<&PlanNode> {
        match &self.operation {
            Operation::Scan { .. } | Operation::Periods { .. } => Vec::new(),
            Operation::Join { left, right, .. } => vec![left, right],
            Operation::SemiJoin {
                input,
                related_all,
                related_visible,
                ..
            } => vec![input, related_all, related_visible],
            Operation::Population { input, periods, .. } => vec![input, periods],
            Operation::Snapshot { input, rule } => {
                let mut inputs = vec![input.as_ref()];
                inputs.extend(rule.inputs());
                inputs
            }
            Operation::Stitch { branches, .. } => branches.iter().collect(),
            Operation::Filter { input, .. }
            | Operation::Deduplicate { input, .. }
            | Operation::Aggregate { input, .. }
            | Operation::Project { input, .. }
            | Operation::Sort { input, .. }
            | Operation::Limit { input, .. } => vec![input],
        }
    }
}

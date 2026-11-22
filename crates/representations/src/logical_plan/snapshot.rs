//! The semi-additive rule a `Snapshot` node applies (E4-3.13, ADD-3.2, E4-7).

use std::num::NonZeroU64;

use menura_contracts::types::common::Grain;
use menura_model::expression::Expression;
use menura_model::internal_model::{
    NormalizedName, NullHandling, SnapshotAggregation, StaleHandling, UnexpectedHandling,
};

use super::PlanNode;
use super::items::{AggregateComponent, NamedExpression};
use crate::names::ColumnName;

/// How a `Snapshot` node reduces the rows of its input to one value per (result group,
/// entity) (E4-3.13, E4-7.1). Every expression is over the columns of the node's `input`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotRule {
    /// The semi-additive metric, by normalized name, which the runtime errors name (ADD-7).
    pub metric: NormalizedName,
    /// The result group: the `Aggregate` keys of the branch, after time-grain truncation, each
    /// with its column name.
    pub keys: Vec<NamedExpression>,
    /// The fields that identify each snapshot entity, in the order the metric writes them,
    /// each with its column name.
    pub entity: Vec<NamedExpression>,
    /// The components, evaluated per (result group, entity, period of `over`) (E4-7.1).
    pub components: Vec<AggregateComponent>,
    /// How an entity's values are combined across `over` (ADD-3.2).
    pub aggregation: SnapshotAggregation,
    /// What a NULL value means (ADD-4.5).
    pub nulls: NullHandling,
    /// The field across which the metric is not summed, with what it needs.
    pub over: SnapshotOver,
}

/// The `over` field of a semi-additive metric (ADD-3.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotOver {
    /// A time `over` (ADD-5.1.5): the rule follows the periods of a population.
    Time(TimeSnapshot),
    /// Any other `over`: its value, over the columns of `input`.
    Field(Expression),
}

/// A time `over`: its periods, its expected entities and what missing and unexpected entities
/// do (ADD-3.2, ADD-4.2–ADD-4.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeSnapshot {
    /// `over` truncated to `grain`, over the columns of `input`: the period of a row.
    pub period: Expression,
    /// The `snapshot_grain`.
    pub grain: Grain,
    /// The key of `keys` that is `over` (or a coarser grain of it), when the query groups by
    /// it: it bounds each result row's unit of periods (ADD-4.7). `None` makes the whole
    /// queried range one unit (ADD-4.10).
    pub unit: Option<UnitKey>,
    /// The expected (entity, period) pairs: the output of a `Population` node (E4-3.12),
    /// whose `entity` columns pair, in order, with `entity` of the rule.
    pub expected: Box<PlanNode>,
    /// What an expected entity without a row contributes (ADD-4.3).
    pub missing: SnapshotMissing,
    /// What a row of an entity the population does not expect does (ADD-4.4).
    pub unexpected: UnexpectedHandling,
}

/// The key column of `over` in the result group, with its query grain (ADD-4.7, E4-7.8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitKey {
    /// The key column, one of the rule's `keys`.
    pub column: ColumnName,
    /// The grain of that key: the query grain of `over`, never finer than `snapshot_grain`
    /// (ADD-4.9).
    pub grain: Grain,
}

/// The `missing` value of a time `over`, with the rows it reads (ADD-4.3, E4-7.5, E4-7.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotMissing {
    /// `exclude`: a missing entity contributes nothing.
    Exclude,
    /// `carry_forward`: a missing entity reuses its latest known value.
    CarryForward {
        /// The rows that carried values are found in: the plan of `input` without the
        /// `date_range` predicate on `over` (E4-7.5), with the columns of `input`.
        carried: Box<PlanNode>,
        /// The oldest value carried, in `snapshot_grain` periods.
        max_carry_forward: NonZeroU64,
        /// What an older value does.
        on_stale: StaleHandling,
    },
    /// `zero`: a missing entity contributes 0.
    Zero {
        /// The rows added for missing entities (E4-7.6): each expected (entity, period)
        /// pair as a home row, its `entity` fields from the population and NULL for every
        /// other home field, joined like a home row and then filtered by the query filters,
        /// with the columns of `input` followed by the column of its period.
        added: Box<PlanNode>,
    },
    /// `error`: a missing entity fails at execution (`RADD-01`).
    Error,
}

impl SnapshotRule {
    /// The plans the rule reads besides the node's `input`, in order: `expected`, then the
    /// `carried` or `added` rows. None for a non-time `over`.
    #[must_use]
    pub fn inputs(&self) -> Vec<&PlanNode> {
        let SnapshotOver::Time(time) = &self.over else {
            return Vec::new();
        };
        let mut inputs = vec![time.expected.as_ref()];
        match &time.missing {
            SnapshotMissing::CarryForward { carried, .. } => inputs.push(carried),
            SnapshotMissing::Zero { added } => inputs.push(added),
            SnapshotMissing::Exclude | SnapshotMissing::Error => {}
        }
        inputs
    }
}

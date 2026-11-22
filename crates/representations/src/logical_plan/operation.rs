//! The operations of plan nodes (E4-3 table).

use menura_contracts::types::common::Grain;
use menura_model::expression::{DateValue, Expression};
use menura_model::internal_model::NormalizedName;

use super::PlanNode;
use super::items::{
    AggregateComponent, JoinCondition, JoinKind, NamedExpression, SortItem, StitchColumn,
};
use super::population::PopulationRule;
use super::snapshot::SnapshotRule;
use crate::names::{ColumnName, ColumnPrefix};

/// What a plan node does (E4-3). Every node of the table is defined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    /// `Scan(relation)`: read the catalog relation of a dataset.
    Scan {
        /// The dataset whose relation is read.
        dataset: NormalizedName,
        /// The column of the synthesized row identity, a row number over the relation
        /// (E4-4.0f): set only on the home `Scan` of a branch with a `Deduplicate`, and then
        /// the last column of the schema.
        row_identity: Option<ColumnName>,
    },
    /// `Join(left, right, relationship, kind)`: join along one relationship. Its output is
    /// the columns of `left`, then those of `right`.
    Join {
        /// The left input: the home `Scan`, or the previous `Join`.
        left: Box<PlanNode>,
        /// The right input: the `Scan` of the join instance the step reaches.
        right: Box<PlanNode>,
        /// The relationship joined along.
        relationship: NormalizedName,
        /// `left`, unless evidence allows `inner` (set only in E5).
        kind: JoinKind,
        /// The equalities of the relationship's column pairs, in the order written.
        on: Vec<JoinCondition>,
        /// The prefix of every column of `right` (E6-4.12).
        right_prefix: ColumnPrefix,
    },
    /// `Filter(input, predicate)`: keep the rows where the predicate is TRUE.
    Filter {
        /// The input.
        input: Box<PlanNode>,
        /// The predicate.
        predicate: Expression,
    },
    /// `SemiJoin`: keep the rows of `input` that are visible along one relationship to D'
    /// (E4-3.4, E4-6.5a): a row is kept only if it has at least one match in `related_all`,
    /// and its number of matches in `related_visible` equals its number of matches in
    /// `related_all`. A NULL key never matches. Its output is the columns of `input`.
    SemiJoin {
        /// The rows of the dataset D.
        input: Box<PlanNode>,
        /// The relationship of the step from D to D'.
        relationship: NormalizedName,
        /// The key equalities: `left` is a column of `input`, `right` the paired column of
        /// `related_all` and of `related_visible`, in the order written.
        on: Vec<JoinCondition>,
        /// The unfiltered `Scan` of D'.
        related_all: Box<PlanNode>,
        /// The plan of the visible rows of D': its own `Filter` and `SemiJoin` nodes over a
        /// `Scan` of D'.
        related_visible: Box<PlanNode>,
    },
    /// `Deduplicate(input, group, identity)`: keep one row per (`group` values, `identity`)
    /// pair, NULLs compared equal (E4-3.5). Its output is the columns of `input`.
    Deduplicate {
        /// The input.
        input: Box<PlanNode>,
        /// The `Aggregate` key expressions, after time-grain truncation, over the columns of
        /// `input` (E4-4.0f).
        group: Vec<Expression>,
        /// The columns that identify a home row: the synthesized row identity (E4-4.0f).
        identity: Vec<ColumnName>,
    },
    /// `Aggregate(input, keys, components)`: group and evaluate components.
    Aggregate {
        /// The input.
        input: Box<PlanNode>,
        /// The grouping keys, each with its column name.
        keys: Vec<NamedExpression>,
        /// The components evaluated per group.
        components: Vec<AggregateComponent>,
    },
    /// `Stitch(branches, keys)`: combine branches by dimension keys (E4-5). Its output is
    /// the key columns, then `columns`, in order.
    Stitch {
        /// The branch plans, in branch order (E3-4.3), each ending with its `Aggregate`.
        branches: Vec<PlanNode>,
        /// The dimension key columns, which every branch outputs, compared with NULLs equal
        /// (E4-5.0a).
        keys: Vec<ColumnName>,
        /// The component columns, in branch order and, within a branch, in component order,
        /// each with its branch and empty-input value (E4-5.0b).
        columns: Vec<StitchColumn>,
    },
    /// `Project(input, expressions)`: compute the output columns.
    Project {
        /// The input.
        input: Box<PlanNode>,
        /// The output columns, in order, each with its expression.
        expressions: Vec<NamedExpression>,
    },
    /// `Population(dataset, population, grain, periods)`: the expected (entity, period) pairs
    /// of `population` at `grain` over `periods` (E4-3.12, POP-3), one row per pair. Its
    /// output is the `entity` columns of `rule`, then `period`.
    Population {
        /// The visible rows V(D) of `dataset`: its `Scan` under its own row-policy `Filter`
        /// and `SemiJoin` nodes (E4-6.5, E2-4.9).
        input: Box<PlanNode>,
        /// The dataset that declares the population.
        dataset: NormalizedName,
        /// The population.
        population: NormalizedName,
        /// The grain of the periods: the `snapshot_grain` of the metric (POP-3.1).
        grain: Grain,
        /// The query's unit of periods (ADD-4.7): one row per period, each the start of a
        /// period at `grain`.
        periods: Box<PlanNode>,
        /// The column of `periods` that holds the period.
        period: ColumnName,
        /// When an entity is expected, over the columns of `input`.
        rule: PopulationRule,
    },
    /// `Periods(from, to, grain)`: one row per period start at `grain` from the period of
    /// `from` to the period of `to`, both ends included (E4-3.14): the unit of periods of a
    /// query with a `date_range` on a semi-additive `over` (ADD-4.7). It reads no input. Its
    /// output is one column, the period start.
    Periods {
        /// The first day of the range.
        from: DateValue,
        /// The last day of the range.
        to: DateValue,
        /// The grain of the periods.
        grain: Grain,
    },
    /// `Snapshot(input, expected, rule)`: reduce the rows of `input` to one value per (result
    /// group, entity) under the semi-additive `rule` (E4-3.13, E4-7.1). With a time `over`,
    /// `expected` is the `Population` output the rule holds. Its output is the `keys` columns
    /// of `rule`, then its `entity` columns, then its `components` columns, one row per
    /// (result group, entity) with a value.
    Snapshot {
        /// The rows of the branch inside the query: its joins and query filters applied
        /// (E4-4.0a–E4-4.0d).
        input: Box<PlanNode>,
        /// The semi-additive rule, with the expected entities and the other rows it reads.
        rule: SnapshotRule,
    },
    /// `Sort(input, items)`: order the rows.
    Sort {
        /// The input.
        input: Box<PlanNode>,
        /// The ordering terms, in order.
        items: Vec<SortItem>,
    },
    /// `Limit(input, n)`: keep at most `rows` rows.
    Limit {
        /// The input.
        input: Box<PlanNode>,
        /// The maximum number of rows.
        rows: u64,
    },
}

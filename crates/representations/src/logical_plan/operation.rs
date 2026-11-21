//! The operations of plan nodes (E4-3 table).

use menura_model::expression::Expression;
use menura_model::internal_model::NormalizedName;

use super::PlanNode;
use super::items::{
    AggregateComponent, JoinCondition, JoinKind, NamedExpression, SortItem, StitchColumn,
};
use crate::names::{ColumnName, ColumnPrefix};
use crate::paths::JoinStep;

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
    /// `SemiJoin(input, path, predicate)`: keep the rows whose related row along `path`
    /// exists and satisfies `predicate`.
    SemiJoin {
        /// The input.
        input: Box<PlanNode>,
        /// The steps to the related row, in order.
        path: Vec<JoinStep>,
        /// The predicate on the related row.
        predicate: Expression,
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

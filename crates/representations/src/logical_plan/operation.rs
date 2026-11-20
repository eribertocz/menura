//! The operations of plan nodes (E4-3 table).

use menura_model::expression::Expression;
use menura_model::internal_model::NormalizedName;

use super::PlanNode;
use super::items::{AggregateComponent, JoinKind, NamedExpression, SortItem};
use crate::names::ColumnName;
use crate::paths::JoinStep;

/// What a plan node does (E4-3). Every node of the table is defined; 0.3.0 builds only
/// `Scan`, `Filter`, `Aggregate`, `Project`, `Sort` and `Limit` (VS-1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    /// `Scan(relation)`: read the catalog relation of a dataset.
    Scan {
        /// The dataset whose relation is read.
        dataset: NormalizedName,
    },
    /// `Join(left, right, relationship, kind)`: join along one relationship.
    Join {
        /// The left input.
        left: Box<PlanNode>,
        /// The right input.
        right: Box<PlanNode>,
        /// The relationship joined along.
        relationship: NormalizedName,
        /// `left`, unless evidence allows `inner` (set only in E5).
        kind: JoinKind,
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
    /// `Deduplicate(input, group, identity)`: keep one row per (dimension key tuple, home row
    /// identity) pair.
    Deduplicate {
        /// The input.
        input: Box<PlanNode>,
        /// The columns of the dimension key tuple.
        group: Vec<ColumnName>,
        /// The columns that identify a home row.
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
    /// `Stitch(branches, keys)`: combine branches by dimension keys (E4-5).
    Stitch {
        /// The branch plans, in branch order (E3-4.3).
        branches: Vec<PlanNode>,
        /// The dimension key columns.
        keys: Vec<ColumnName>,
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

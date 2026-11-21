//! The values plan nodes carry: join kinds and conditions, named expressions, components and sort items.

use menura_contracts::types::query::{Direction, NullsOrder};
use menura_model::expression::{AggregateCall, Expression};

use crate::names::ColumnName;

/// The kind of a join (E4-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JoinKind {
    /// A left outer join: the default.
    Left,
    /// An inner join, set only in E5 when evidence allows it.
    Inner,
}

/// One equality of a `Join`: the `left` column equals the `right` column (E4-3.2). A NULL on
/// either side never matches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinCondition {
    /// The column of the left input: a `from_columns` field of the relationship.
    pub left: ColumnName,
    /// The column of the right input: the paired `to_columns` field.
    pub right: ColumnName,
}

/// An expression with the name of the column it computes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedExpression {
    /// The column it computes.
    pub name: ColumnName,
    /// The expression.
    pub expression: Expression,
}

/// A metric component evaluated by an `Aggregate` node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateComponent {
    /// The column it computes.
    pub name: ColumnName,
    /// The aggregate call.
    pub call: AggregateCall,
    /// Its `FILTER (WHERE …)` condition, when it has one.
    pub filter: Option<Expression>,
}

/// One ordering term of a `Sort` node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortItem {
    /// The column ordered by.
    pub column: ColumnName,
    /// The direction.
    pub direction: Direction,
    /// Where NULLs go.
    pub nulls: NullsOrder,
}

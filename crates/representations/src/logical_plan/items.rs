//! The values plan nodes carry: join kinds, named expressions, components and sort items.

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

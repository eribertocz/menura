//! The values plan nodes carry: join kinds and conditions, named expressions, components and sort items.

use menura_contracts::types::query::{Direction, NullsOrder};
use menura_model::expression::{AggregateCall, Expression, Literal};
use menura_model::internal_model::{DivisionByZero, NormalizedName};

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
    /// In a `Project`, the derived metric whose formula the expression is, when it is one:
    /// its divisions follow that metric's `on_division_by_zero` (DER-3.1, E6-4.6a). `None`
    /// everywhere else, where a division by zero yields NULL (E0F-6.1).
    pub derived: Option<DerivedFormula>,
}

/// The derived metric whose formula a `Project` column computes (DER-3.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedFormula {
    /// The normalized name of the derived metric, which `RDER-01` states (E6-4.6a).
    pub metric: NormalizedName,
    /// Its `on_division_by_zero`.
    pub on_division_by_zero: DivisionByZero,
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

/// A component column of a `Stitch` node: the branch that computes it and its value for a
/// key that branch has no row for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StitchColumn {
    /// The component column, as the branch outputs it.
    pub name: ColumnName,
    /// The position of the branch that computes it, from 0, in branch order.
    pub branch: usize,
    /// The component's value over no rows (E4-5.0b): `0` for `COUNT`; `None` is NULL.
    pub empty_value: Option<Literal>,
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

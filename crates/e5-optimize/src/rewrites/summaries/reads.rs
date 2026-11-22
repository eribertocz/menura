//! The columns of the home `Scan` that a branch reads above it, and how it reads them.

use menura_model::expression::{DateUnit, Expression};
use menura_representations::bound_query::BoundQuery;
use menura_representations::logical_plan::JoinCondition;
use menura_representations::names::ColumnName;

use super::branch::BranchShape;
use super::expression_map::columns_read;

/// How a branch reads a column of its home `Scan`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReadKind {
    /// By a query dimension, filter or join, other than as below.
    Plain,
    /// As an `Aggregate` key `DATE_TRUNC(unit, column)`: a query dimension with a time grain.
    Truncated(DateUnit),
    /// By the `date_range` predicate.
    DateRange,
    /// By a row policy of the home dataset: its own condition or a propagation key.
    Policy,
}

/// One read of a column of the home `Scan`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Read {
    /// The column.
    pub(super) column: ColumnName,
    /// How it is read.
    pub(super) kind: ReadKind,
}

/// Every read of a column of the home `Scan` by the nodes of `shape` above it, for `query`.
pub(super) fn branch_reads(shape: &BranchShape<'_>, query: &BoundQuery) -> Vec<Read> {
    let mut reads = Vec::new();
    let mut add = |expression: &Expression, kind: ReadKind| {
        reads.extend(
            columns_read(expression)
                .into_iter()
                .map(|column| Read { column, kind }),
        );
    };
    if let Some(policy) = shape.policy_filter {
        add(policy.value, ReadKind::Policy);
    }
    let (filters, date_range) = shape.query_terms(query);
    if let Some(filters) = filters {
        add(filters, ReadKind::Plain);
    }
    if let Some(date_range) = date_range {
        add(date_range, ReadKind::DateRange);
    }
    for dimension in shape.computed_dimensions() {
        add(&dimension.expression, ReadKind::Plain);
    }
    for key in shape.keys {
        match &key.expression {
            Expression::DateTrunc { unit, operand }
                if matches!(**operand, Expression::Reference(_)) =>
            {
                add(operand, ReadKind::Truncated(*unit));
            }
            expression => add(expression, ReadKind::Plain),
        }
    }
    for semi_join in &shape.semi_joins {
        reads.extend(key_reads(semi_join.value, ReadKind::Policy));
    }
    for join in &shape.joins {
        reads.extend(key_reads(join.value, ReadKind::Plain));
    }
    reads.retain(|read| shape.is_home_column(&read.column));
    reads
}

/// The reads of the `left` columns of `conditions`, the `from_columns` of a relationship.
fn key_reads(conditions: &[JoinCondition], kind: ReadKind) -> impl Iterator<Item = Read> + '_ {
    conditions.iter().map(move |condition| Read {
        column: condition.left.clone(),
        kind,
    })
}

//! What an expression of the statement contains: field references, `AGG(…)` terms, and the
//! constructs the SQL form does not accept (E1-4 "SELECT items", "WHERE", E1-4.8).

use menura_model::expression::{AggregateCall, Expression};

/// The kinds of term an expression contains, outside and inside its `AGG(…)` terms.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Contents {
    /// A field reference outside every `AGG(…)` term.
    pub(super) field_reference: bool,
    /// An `AGG(…)` or `MEASURE(…)` term.
    pub(super) metric_aggregate: bool,
    /// The first construct the SQL form does not accept: an aggregate function such as `SUM`,
    /// or `USER_ATTRIBUTE`.
    pub(super) unsupported: Option<&'static str>,
}

/// The contents of `expression`.
pub(super) fn contents(expression: &Expression) -> Contents {
    let mut found = Contents::default();
    let mut pending = vec![expression];
    while let Some(expression) = pending.pop() {
        match expression {
            Expression::Reference(_) => found.field_reference = true,
            Expression::MetricAggregate(_) => found.metric_aggregate = true,
            Expression::Aggregate { .. } => {
                found
                    .unsupported
                    .get_or_insert("an aggregate function; use AGG(metric)");
            }
            Expression::UserAttribute(_) => {
                found.unsupported.get_or_insert("USER_ATTRIBUTE");
            }
            _ => {}
        }
        pending.extend(children(expression));
    }
    found
}

/// The direct subexpressions of `expression`.
fn children(expression: &Expression) -> Vec<&Expression> {
    match expression {
        Expression::Literal(_)
        | Expression::Reference(_)
        | Expression::MetricAggregate(_)
        | Expression::UserAttribute(_)
        | Expression::CurrentDate
        | Expression::CurrentTime
        | Expression::CurrentTimestamp => Vec::new(),
        Expression::Unary { operand, .. }
        | Expression::IsNull { operand, .. }
        | Expression::Cast { operand, .. }
        | Expression::Extract { operand, .. }
        | Expression::DateTrunc { operand, .. } => vec![operand],
        Expression::Binary { left, right, .. } | Expression::IsDistinctFrom { left, right, .. } => {
            vec![left, right]
        }
        Expression::InList { operand, list, .. } => {
            std::iter::once(operand.as_ref()).chain(list).collect()
        }
        Expression::Between {
            operand, low, high, ..
        } => vec![operand, low, high],
        Expression::Like {
            operand, pattern, ..
        } => vec![operand, pattern],
        Expression::Case {
            operand,
            branches,
            otherwise,
        } => operand
            .as_deref()
            .into_iter()
            .chain(
                branches
                    .iter()
                    .flat_map(|branch| [&branch.condition, &branch.result]),
            )
            .chain(otherwise.as_deref())
            .collect(),
        Expression::DateAdd {
            amount, operand, ..
        } => vec![amount, operand],
        Expression::DateDiff { start, end, .. } => vec![start, end],
        Expression::Function { arguments, .. } => arguments.iter().collect(),
        Expression::Aggregate { call, filter } => aggregate_children(call)
            .into_iter()
            .chain(filter.as_deref())
            .collect(),
    }
}

fn aggregate_children(call: &AggregateCall) -> Vec<&Expression> {
    match call {
        AggregateCall::CountRows => Vec::new(),
        AggregateCall::Simple { argument, .. } | AggregateCall::Distinct { argument, .. } => {
            vec![argument]
        }
        AggregateCall::Percentile {
            fraction, ordering, ..
        } => vec![fraction, ordering],
    }
}

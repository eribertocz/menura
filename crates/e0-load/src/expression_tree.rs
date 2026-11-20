//! The one walk over normalized expression trees shared by the substages of E0: the direct
//! subexpressions of a node in written order, and every node before its subexpressions.

use menura_model::expression::{AggregateCall, Expression};

/// Every node of `expression`, each before its subexpressions, in written order.
pub(crate) fn nodes(expression: &Expression) -> Vec<&Expression> {
    let mut found = Vec::new();
    collect_nodes(expression, &mut found);
    found
}

fn collect_nodes<'tree>(expression: &'tree Expression, found: &mut Vec<&'tree Expression>) {
    found.push(expression);
    for child in children(expression) {
        collect_nodes(child, found);
    }
}

/// The direct subexpressions of `expression`, in the order they are written; for an
/// aggregate, its arguments and then its `FILTER`.
pub(crate) fn children(expression: &Expression) -> Vec<&Expression> {
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
        Expression::Aggregate { call, filter } => aggregate_arguments(call)
            .into_iter()
            .chain(filter.as_deref())
            .collect(),
    }
}

/// The arguments of an aggregate call, in written order: none for `COUNT(*)`.
pub(crate) fn aggregate_arguments(call: &AggregateCall) -> Vec<&Expression> {
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

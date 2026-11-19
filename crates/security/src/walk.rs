//! The direct subexpressions of an expression tree, in written order.

use semantic_compiler_model::expression::{AggregateCall, Expression};

/// The direct subexpressions of `expression`, in the order they are written.
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

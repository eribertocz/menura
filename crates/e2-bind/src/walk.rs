//! The nodes of an expression tree, in written order, and its references, mutably.

use menura_model::expression::{AggregateCall, Expression, Reference};

/// Every node of `expression`, in written order: each node before its subexpressions.
pub(crate) fn nodes(expression: &Expression) -> Vec<&Expression> {
    let mut found = Vec::new();
    let mut pending = vec![expression];
    while let Some(node) = pending.pop() {
        found.push(node);
        let mut next = children(node);
        next.reverse();
        pending.extend(next);
    }
    found
}

/// The direct subexpressions of `expression`, in written order.
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

/// The subexpressions of an aggregate call, in written order.
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

/// Calls `visit` on every reference of `expression` and on the metric of every `AGG(…)`, in
/// written order: the order of [`nodes`].
pub(crate) fn for_each_reference_mut(
    expression: &mut Expression,
    visit: &mut impl FnMut(&mut Reference),
) {
    if let Expression::Reference(reference) | Expression::MetricAggregate(reference) = expression {
        visit(reference);
        return;
    }
    for child in children_mut(expression) {
        for_each_reference_mut(child, visit);
    }
}

/// The direct subexpressions of `expression`, in written order, mutably: the order of
/// [`children`].
fn children_mut(expression: &mut Expression) -> Vec<&mut Expression> {
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
        | Expression::DateTrunc { operand, .. } => vec![operand.as_mut()],
        Expression::Binary { left, right, .. } | Expression::IsDistinctFrom { left, right, .. } => {
            vec![left.as_mut(), right.as_mut()]
        }
        Expression::InList { operand, list, .. } => std::iter::once(operand.as_mut())
            .chain(list.iter_mut())
            .collect(),
        Expression::Between {
            operand, low, high, ..
        } => vec![operand.as_mut(), low.as_mut(), high.as_mut()],
        Expression::Like {
            operand, pattern, ..
        } => vec![operand.as_mut(), pattern.as_mut()],
        Expression::Case {
            operand,
            branches,
            otherwise,
        } => operand
            .as_deref_mut()
            .into_iter()
            .chain(
                branches
                    .iter_mut()
                    .flat_map(|branch| [&mut branch.condition, &mut branch.result]),
            )
            .chain(otherwise.as_deref_mut())
            .collect(),
        Expression::DateAdd {
            amount, operand, ..
        } => vec![amount.as_mut(), operand.as_mut()],
        Expression::DateDiff { start, end, .. } => vec![start.as_mut(), end.as_mut()],
        Expression::Function { arguments, .. } => arguments.iter_mut().collect(),
        Expression::Aggregate { call, filter } => aggregate_children_mut(call)
            .into_iter()
            .chain(filter.as_deref_mut())
            .collect(),
    }
}

/// The subexpressions of an aggregate call, in written order, mutably.
fn aggregate_children_mut(call: &mut AggregateCall) -> Vec<&mut Expression> {
    match call {
        AggregateCall::CountRows => Vec::new(),
        AggregateCall::Simple { argument, .. } | AggregateCall::Distinct { argument, .. } => {
            vec![argument.as_mut()]
        }
        AggregateCall::Percentile {
            fraction, ordering, ..
        } => vec![fraction.as_mut(), ordering.as_mut()],
    }
}

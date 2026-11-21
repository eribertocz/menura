//! The `AGG(…)` terms of a derived formula, in written order (DER-2.1).

use menura_model::expression::{Expression, Reference};

/// The references of the `AGG(…)` terms of `formula`, in written order, each as often as it
/// is written. Aggregate calls are not searched: a derived formula has none (DER-2.3).
pub(crate) fn metric_inputs(formula: &Expression) -> Vec<&Reference> {
    let mut found = Vec::new();
    let mut pending = vec![formula];
    while let Some(expression) = pending.pop() {
        if let Expression::MetricAggregate(reference) = expression {
            found.push(reference);
        }
        let mut children = subexpressions(expression);
        children.reverse();
        pending.extend(children);
    }
    found
}

/// The direct subexpressions of `expression`, in written order.
fn subexpressions(expression: &Expression) -> Vec<&Expression> {
    match expression {
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
        Expression::Literal(_)
        | Expression::Reference(_)
        | Expression::MetricAggregate(_)
        | Expression::UserAttribute(_)
        | Expression::CurrentDate
        | Expression::CurrentTime
        | Expression::CurrentTimestamp
        | Expression::Aggregate { .. } => Vec::new(),
    }
}

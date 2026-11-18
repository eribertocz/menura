//! The structural conditions of `E0C-23` (POP-4.4, POP-4.5): the constructs a population
//! condition may not contain, found by walking the whole tree, whatever the types of the
//! fields it reads.

use semantic_compiler_model::expression::{AggregateCall, Expression, Reference};

/// The first construct of `condition`, in written order, that a row-level condition does not
/// allow: a qualified reference, an aggregate, `AGG(…)` or `USER_ATTRIBUTE(…)`; described for
/// a message.
pub(super) fn structural_problem(condition: &Expression) -> Option<String> {
    nodes(condition).into_iter().find_map(|node| match node {
        Expression::Reference(Reference {
            qualifier: Some(qualifier),
            name,
        }) => Some(format!(
            "the qualified reference `{}`.`{}`; a field of the dataset is read by its bare name",
            qualifier.text(),
            name.text()
        )),
        Expression::Aggregate { .. } => Some("an aggregate".to_owned()),
        Expression::MetricAggregate(_) => Some("AGG(…)".to_owned()),
        Expression::UserAttribute(_) => Some("USER_ATTRIBUTE(…)".to_owned()),
        _ => None,
    })
}

/// The bare references of `condition`, in written order: the fields of the dataset it reads.
pub(super) fn bare_references(condition: &Expression) -> Vec<&Reference> {
    nodes(condition)
        .into_iter()
        .filter_map(|node| match node {
            Expression::Reference(reference) if reference.qualifier.is_none() => Some(reference),
            _ => None,
        })
        .collect()
}

/// Every node of `expression`, each before its subexpressions, in written order; the derived
/// formulas are walked with it too.
pub(in crate::typing) fn nodes(expression: &Expression) -> Vec<&Expression> {
    let mut found = vec![expression];
    for child in children(expression) {
        found.extend(nodes(child));
    }
    found
}

/// The direct subexpressions of `expression`, in the order they are written.
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

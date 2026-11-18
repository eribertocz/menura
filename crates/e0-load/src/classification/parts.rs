//! Finds the parts of a metric expression that classification reads: its aggregates, with the
//! references of their arguments and of their `FILTER`, and the references outside every
//! aggregate (E0G-1.1, E0G-2.1).

use semantic_compiler_model::expression::{AggregateCall, Expression, Reference};

/// The aggregates and unaggregated references of one metric expression.
#[derive(Debug, Default)]
pub(super) struct ExpressionParts<'tree> {
    /// Every aggregate, in written order.
    pub(super) aggregates: Vec<AggregateParts<'tree>>,
    /// Every field or column reference outside every aggregate, in written order.
    pub(super) unaggregated_references: Vec<&'tree Reference>,
}

/// One aggregate and the references it reads.
#[derive(Debug)]
pub(super) struct AggregateParts<'tree> {
    /// The call.
    pub(super) call: &'tree AggregateCall,
    /// Its `FILTER (WHERE …)` condition, if written.
    pub(super) filter: Option<&'tree Expression>,
    /// The references of its arguments, in written order.
    pub(super) argument_references: Vec<&'tree Reference>,
    /// The references of its `FILTER (WHERE …)`, in written order.
    pub(super) filter_references: Vec<&'tree Reference>,
}

/// The parts of `expression`.
///
/// An aggregate is not searched for further aggregates: a nested aggregate is `E0F-07`, and
/// such a metric is not usable after E0.f (E0G-2.6, E0G-2.8).
pub(super) fn expression_parts(expression: &Expression) -> ExpressionParts<'_> {
    let mut parts = ExpressionParts::default();
    collect(expression, &mut parts);
    parts
}

fn collect<'tree>(expression: &'tree Expression, parts: &mut ExpressionParts<'tree>) {
    match expression {
        Expression::Aggregate { call, filter } => {
            let mut argument_references = Vec::new();
            for argument in arguments(call) {
                references(argument, &mut argument_references);
            }
            let mut filter_references = Vec::new();
            if let Some(filter) = filter {
                references(filter, &mut filter_references);
            }
            parts.aggregates.push(AggregateParts {
                call,
                filter: filter.as_deref(),
                argument_references,
                filter_references,
            });
        }
        Expression::Reference(reference) => parts.unaggregated_references.push(reference),
        _ => {
            for child in children(expression) {
                collect(child, parts);
            }
        }
    }
}

/// Every field or column reference of `expression`, in written order.
fn references<'tree>(expression: &'tree Expression, found: &mut Vec<&'tree Reference>) {
    if let Expression::Reference(reference) = expression {
        found.push(reference);
    }
    for child in children(expression) {
        references(child, found);
    }
}

/// The arguments of an aggregate call: none for `COUNT(*)`.
fn arguments(call: &AggregateCall) -> Vec<&Expression> {
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
        Expression::Aggregate { call, filter } => arguments(call)
            .into_iter()
            .chain(filter.as_deref())
            .collect(),
    }
}

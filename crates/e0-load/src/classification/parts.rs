//! Finds the parts of a metric expression that classification reads: its aggregates, with the
//! references of their arguments and of their `FILTER`, and the references outside every
//! aggregate (E0G-1.1, E0G-2.1).

use menura_model::expression::{AggregateCall, Expression, Reference};

use crate::expression_tree::{aggregate_arguments, children, nodes};

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
            for argument in aggregate_arguments(call) {
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
    found.extend(nodes(expression).into_iter().filter_map(|node| match node {
        Expression::Reference(reference) => Some(reference),
        _ => None,
    }));
}

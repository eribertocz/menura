//! The references an expression makes: the plan columns, catalog columns or fields it names.

use menura_contracts::normalize_unquoted_identifier;
use menura_model::expression::{AggregateCall, Expression, Identifier, Reference};
use menura_model::internal_model::NormalizedName;
use menura_representations::logical_plan::AggregateComponent;

/// Every reference `expression` makes, `AGG(…)` references included, in written order.
pub(super) fn references(expression: &Expression) -> Vec<&Reference> {
    let mut found = Vec::new();
    add_references(expression, &mut found);
    found
}

/// Every reference `component` makes, in its call and then its filter.
pub(super) fn component_references(component: &AggregateComponent) -> Vec<&Reference> {
    let mut found = Vec::new();
    add_call_references(&component.call, &mut found);
    if let Some(filter) = &component.filter {
        add_references(filter, &mut found);
    }
    found
}

/// The names of the plan columns `expression` reads: plan expressions reference columns by
/// unqualified name (E4-3.11).
pub(super) fn columns_read(expression: &Expression) -> Vec<&str> {
    references(expression)
        .into_iter()
        .map(|reference| reference.name.text())
        .collect()
}

/// The references `AGG(…)` makes anywhere in `expression`, in written order.
pub(super) fn aggregated_references(expression: &Expression) -> Vec<&Reference> {
    let mut found = Vec::new();
    visit(expression, &mut |current| {
        if let Expression::MetricAggregate(reference) = current {
            found.push(reference);
        }
    });
    found
}

fn add_references<'expression>(
    expression: &'expression Expression,
    found: &mut Vec<&'expression Reference>,
) {
    visit(expression, &mut |current| {
        if let Expression::Reference(reference) | Expression::MetricAggregate(reference) = current {
            found.push(reference);
        }
    });
}

fn add_call_references<'call>(call: &'call AggregateCall, found: &mut Vec<&'call Reference>) {
    for argument in call_arguments(call) {
        add_references(argument, found);
    }
}

/// Calls `action` on `expression` and on every expression inside it, aggregate arguments
/// included, in written order.
fn visit<'expression>(
    expression: &'expression Expression,
    action: &mut impl FnMut(&'expression Expression),
) {
    action(expression);
    if let Expression::Aggregate { call, .. } = expression {
        for argument in call_arguments(call) {
            visit(argument, action);
        }
    }
    for operand in operands(expression) {
        visit(operand, action);
    }
}

/// The arguments of `call`, in written order.
fn call_arguments(call: &AggregateCall) -> Vec<&Expression> {
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

/// The direct sub-expressions of `expression`, in written order, except the arguments of an
/// aggregate call.
fn operands(expression: &Expression) -> Vec<&Expression> {
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
            let mut operands = vec![operand.as_ref()];
            operands.extend(list);
            operands
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
        } => {
            let mut operands: Vec<&Expression> = operand.iter().map(AsRef::as_ref).collect();
            for branch in branches {
                operands.push(&branch.condition);
                operands.push(&branch.result);
            }
            operands.extend(otherwise.iter().map(AsRef::as_ref));
            operands
        }
        Expression::DateAdd {
            amount, operand, ..
        } => vec![amount, operand],
        Expression::DateDiff { start, end, .. } => vec![start, end],
        Expression::Function { arguments, .. } => arguments.iter().collect(),
        Expression::Aggregate { filter, .. } => filter.iter().map(AsRef::as_ref).collect(),
    }
}

/// The normalized name of `identifier` (E0D-1.1): a regular identifier in upper case, a
/// delimited one as written.
pub(super) fn normalized_name(identifier: &Identifier) -> NormalizedName {
    if identifier.is_delimited() {
        NormalizedName::new(identifier.text())
    } else {
        NormalizedName::new(&normalize_unquoted_identifier(identifier.text()))
    }
}

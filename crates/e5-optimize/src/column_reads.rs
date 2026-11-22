//! The plan columns that expressions and plan nodes read.

use std::collections::BTreeSet;

use menura_model::expression::{AggregateCall, Expression};
use menura_representations::logical_plan::{AggregateComponent, LookupKey, Operation};

/// The names of plan columns, as text.
pub(crate) type ColumnNames = BTreeSet<String>;

/// Adds the columns that `operation` itself reads, not its inputs, to `columns`: the
/// expressions, conditions, keys, identities and sort columns it carries. A `Snapshot` and a
/// `Population` read their own rules, which this does not cover.
pub(crate) fn add_operation_reads(operation: &Operation, columns: &mut ColumnNames) {
    match operation {
        Operation::Join { on, .. }
        | Operation::SemiJoin { on, .. }
        | Operation::FullJoin { on, .. } => {
            columns.extend(on.iter().map(|pair| pair.left.as_str().to_owned()));
        }
        Operation::Filter { predicate, .. } => add_expression_reads(predicate, columns),
        Operation::PeriodRange { coverage, .. } => {
            columns.insert(coverage.as_str().to_owned());
        }
        Operation::Deduplicate {
            group, identity, ..
        } => {
            for expression in group {
                add_expression_reads(expression, columns);
            }
            columns.extend(identity.iter().map(|name| name.as_str().to_owned()));
        }
        Operation::Aggregate {
            keys, components, ..
        } => {
            for key in keys {
                add_expression_reads(&key.expression, columns);
            }
            for component in components {
                add_component_reads(component, columns);
            }
        }
        Operation::Stitch { keys, .. } => {
            columns.extend(keys.iter().map(|key| key.as_str().to_owned()));
        }
        Operation::Lookup { mapping, .. } => {
            columns.extend(mapping.iter().map(|key| match key {
                LookupKey::Equal { result, .. } => result.as_str().to_owned(),
            }));
        }
        Operation::Project { expressions, .. } => expressions
            .iter()
            .for_each(|named| add_expression_reads(&named.expression, columns)),
        Operation::Sort { items, .. } => {
            columns.extend(items.iter().map(|item| item.column.as_str().to_owned()));
        }
        Operation::Scan { .. }
        | Operation::Population { .. }
        | Operation::Periods { .. }
        | Operation::Snapshot { .. }
        | Operation::Limit { .. } => {}
    }
}

/// Adds the columns `component` reads, in its call and its filter, to `columns`.
fn add_component_reads(component: &AggregateComponent, columns: &mut ColumnNames) {
    add_call_reads(&component.call, columns);
    if let Some(filter) = &component.filter {
        add_expression_reads(filter, columns);
    }
}

/// Adds the columns `call` reads to `columns`.
fn add_call_reads(call: &AggregateCall, columns: &mut ColumnNames) {
    match call {
        AggregateCall::CountRows => {}
        AggregateCall::Simple { argument, .. } | AggregateCall::Distinct { argument, .. } => {
            add_expression_reads(argument, columns);
        }
        AggregateCall::Percentile {
            fraction, ordering, ..
        } => {
            add_expression_reads(fraction, columns);
            add_expression_reads(ordering, columns);
        }
    }
}

/// Adds the name of every column `expression` references to `columns`. Plan expressions
/// reference columns by unqualified name (E4-3.11); a qualified reference adds its name too.
pub(crate) fn add_expression_reads(expression: &Expression, columns: &mut ColumnNames) {
    for operand in operands(expression) {
        add_expression_reads(operand, columns);
    }
    match expression {
        Expression::Reference(reference) | Expression::MetricAggregate(reference) => {
            columns.insert(reference.name.text().to_owned());
        }
        Expression::Aggregate { call, .. } => add_call_reads(call, columns),
        _ => {}
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

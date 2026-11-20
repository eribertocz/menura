//! The home datasets of a model metric (E0G-1.1), as `E0C-16` reads them (ADD-5.1.6): the
//! datasets its aggregate arguments read.

use std::collections::BTreeSet;

use menura_model::expression::{AggregateCall, Expression, Reference};

use crate::names::Name;

/// The datasets that the arguments of the aggregates of `expression` read, through their
/// references `dataset.field`.
///
/// An aggregate is not searched for further aggregates: a nested aggregate makes the metric
/// unusable (`E0F-07`). A `FILTER` does not decide the home dataset (E0G-1.1).
pub(super) fn home_datasets(expression: &Expression) -> BTreeSet<Name> {
    let mut found = BTreeSet::new();
    collect(expression, &mut found);
    found
}

fn collect(expression: &Expression, found: &mut BTreeSet<Name>) {
    if let Expression::Aggregate { call, .. } = expression {
        for argument in arguments(call) {
            qualifiers(argument, found);
        }
        return;
    }
    for child in children(expression) {
        collect(child, found);
    }
}

/// The datasets of the references `dataset.field` of `expression`.
fn qualifiers(expression: &Expression, found: &mut BTreeSet<Name>) {
    if let Expression::Reference(Reference {
        qualifier: Some(qualifier),
        ..
    }) = expression
    {
        found.insert(Name::of(qualifier));
    }
    for child in children(expression) {
        qualifiers(child, found);
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

/// The direct subexpressions of `expression`; for an aggregate, its arguments and `FILTER`.
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

#[cfg(test)]
mod tests {
    use super::home_datasets;
    use crate::expressions::{ExpressionPlace, parse_element_expression};

    fn homes(text: &str) -> Vec<String> {
        let tree = parse_element_expression(text, ExpressionPlace::Metric).expect("it parses");
        home_datasets(&tree)
            .iter()
            .map(|name| name.as_str().to_owned())
            .collect()
    }

    #[test]
    fn add_5_1_6_home_datasets_are_read_by_aggregate_arguments() {
        assert_eq!(homes("SUM(debt.balance)"), ["DEBT"]);
        assert_eq!(
            homes("SUM(debt.balance) / COUNT(customers.customer_id)"),
            ["CUSTOMERS", "DEBT"]
        );
        assert_eq!(
            homes("SUM(debt.balance) FILTER (WHERE customers.category = 'A')"),
            ["DEBT"]
        );
        assert!(homes("COUNT(*)").is_empty());
        assert!(homes("1 + 2").is_empty());
        assert_eq!(homes("debt.balance + MAX(\"Debt\".balance)"), ["Debt"]);
    }
}

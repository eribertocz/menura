//! The home datasets of a model metric (E0G-1.1), as `E0C-16` reads them (ADD-5.1.6): the
//! datasets its aggregate arguments read.

use std::collections::BTreeSet;

use menura_model::expression::{Expression, Reference};

use crate::expression_tree::{aggregate_arguments, children, nodes};
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
        for argument in aggregate_arguments(call) {
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
    for node in nodes(expression) {
        if let Expression::Reference(Reference {
            qualifier: Some(qualifier),
            ..
        }) = node
        {
            found.insert(Name::of(qualifier));
        }
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

//! The structural conditions of `E0C-23` (POP-4.4, POP-4.5): the constructs a population
//! condition may not contain, found by walking the whole tree, whatever the types of the
//! fields it reads.

use menura_model::expression::{Expression, Reference};

use crate::expression_tree::nodes;

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

//! `E0C-30`, `E0C-31` and `E0C-35`: what a derived formula may contain (DER-2.3, DER-5.1.3).

use menura_model::expression::{BinaryOperator, Expression, Reference};

use super::super::populations::structure::nodes;
use crate::extensions::ExtensionCode;
use crate::references::derived::{DerivedMetric, DerivedOrigin};

/// The problems of the formula `tree` of `metric`, at most one per code, in the order of the
/// codes: an aggregate function anywhere (`E0C-30`), a field reference anywhere (`E0C-31`),
/// and, for a declared metric without `on_division_by_zero`, a `/` or `%` (`E0C-35`).
pub(super) fn formula_problems(
    tree: &Expression,
    metric: &DerivedMetric,
) -> Vec<(ExtensionCode, String)> {
    let all = nodes(tree);
    let mut problems = Vec::new();
    if all
        .iter()
        .any(|node| matches!(node, Expression::Aggregate { .. }))
    {
        problems.push((
            ExtensionCode::AggregateInFormula,
            format!(
                "the derived formula of `{}` has an aggregate function; it reads metrics with AGG(metric) only",
                metric.name
            ),
        ));
    }
    let references: Vec<String> = all
        .iter()
        .filter_map(|node| match node {
            Expression::Reference(reference) => Some(written(reference)),
            _ => None,
        })
        .collect();
    if !references.is_empty() {
        problems.push((
            ExtensionCode::FieldInFormula,
            format!(
                "the derived formula of `{}` reads the field {}; it reads metrics with AGG(metric) only",
                metric.name,
                references.join(", ")
            ),
        ));
    }
    if metric.origin == DerivedOrigin::Declared(None) && all.iter().any(|node| divides(node)) {
        problems.push((
            ExtensionCode::MissingDivisionRule,
            format!(
                "the derived formula of `{}` divides, and its `derived` has no `on_division_by_zero`",
                metric.name
            ),
        ));
    }
    problems
}

/// Whether `node` is the operator `/` or `%` (DER-5.1.3).
fn divides(node: &Expression) -> bool {
    matches!(
        node,
        Expression::Binary {
            operator: BinaryOperator::Divide | BinaryOperator::Modulo,
            ..
        }
    )
}

/// A reference as written, such as `` `invoices`.`billed` ``.
fn written(reference: &Reference) -> String {
    let name = format!("`{}`", reference.name.text());
    match &reference.qualifier {
        Some(qualifier) => format!("`{}`.{name}", qualifier.text()),
        None => name,
    }
}

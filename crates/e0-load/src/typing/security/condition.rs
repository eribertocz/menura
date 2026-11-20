//! The problems of one row-policy condition (SCY-5.1.6).

use menura_contracts::types::common::LogicalType;
use menura_model::expression::{Expression, Reference};
use menura_security::{AttributeType, ScalarType, misplaced_list_attributes};

use super::{FieldLookup, Fields};
use crate::expression_tree::nodes;
use crate::expressions::{ExpressionPlace, parse_element_expression};
use crate::extensions::security::{RowPolicy, SecurityContextDeclaration};
use crate::extensions::{ExtensionCode, Problems};
use crate::names::Name;
use menura_expressions::typing::{Aggregates, TypingScope, type_expression};
use menura_expressions::typing::{Outcome, Stop};
use menura_expressions::typing::{Typed, big_integer, decimal, describe};

/// The `E0C-46` and `E0C-48` problems of the condition of `policy`, whose attributes are
/// declared by `context`.
pub(super) fn condition_problems(
    fields: &Fields<'_, '_>,
    context: Option<&SecurityContextDeclaration>,
    policy: &RowPolicy,
) -> Problems {
    let invalid = |problem: String| {
        (
            ExtensionCode::InvalidPolicyCondition,
            format!(
                "the condition of the row policy `{}`: {problem}",
                policy.name
            ),
        )
    };
    let tree = match parse_element_expression(&policy.condition, ExpressionPlace::Extension) {
        Ok(tree) => tree,
        Err(diagnostic) => return vec![invalid(diagnostic.error.to_string())],
    };
    let attribute = |name: &str| context.and_then(|context| context.attribute(name));
    let mut problems: Problems = structural_problems(fields, policy, &tree, &attribute)
        .into_iter()
        .map(invalid)
        .collect();
    let misplaced = misplaced_list_attributes(&tree, |name| {
        attribute(name).is_some_and(AttributeType::is_list)
    });
    for name in &misplaced {
        problems.push((
            ExtensionCode::MisplacedListAttribute,
            format!(
                "the condition of the row policy `{}` uses the list attribute `{name}` outside the single item of an IN or NOT IN list",
                policy.name
            ),
        ));
    }
    if problems.is_empty() {
        problems.extend(type_problem(fields, policy, &tree, &attribute).map(invalid));
    }
    problems
}

/// The constructs a condition may not contain, the fields it reads that its dataset does not
/// have, and the attributes it reads that are not declared, in written order (SCY-4.2,
/// SCY-5.1.6).
fn structural_problems(
    fields: &Fields<'_, '_>,
    policy: &RowPolicy,
    tree: &Expression,
    attribute: &impl Fn(&str) -> Option<AttributeType>,
) -> Vec<String> {
    let mut problems = Vec::new();
    for node in nodes(tree) {
        let problem = match node {
            Expression::Reference(Reference {
                qualifier: Some(qualifier),
                name,
            }) => Some(format!(
                "the qualified reference `{}`.`{}`; a field of the dataset is read by its bare name",
                qualifier.text(),
                name.text()
            )),
            Expression::Reference(reference) => matches!(
                fields.lookup(&policy.dataset, &Name::of(&reference.name)),
                FieldLookup::Missing
            )
            .then(|| format!("`{}` is not a field of the dataset", reference.name.text())),
            Expression::Aggregate { .. } => Some("an aggregate".to_owned()),
            Expression::MetricAggregate(_) => Some("AGG(…)".to_owned()),
            Expression::UserAttribute(name) => attribute(name)
                .is_none()
                .then(|| format!("the attribute `{name}` is not declared in security_context")),
            _ => None,
        };
        if let Some(problem) = problem.filter(|problem| !problems.contains(problem)) {
            problems.push(problem);
        }
    }
    problems
}

/// Why `tree` is not boolean, when every field it reads has a type (SCY-5.1.6).
fn type_problem(
    fields: &Fields<'_, '_>,
    policy: &RowPolicy,
    tree: &Expression,
    attribute: &impl Fn(&str) -> Option<AttributeType>,
) -> Option<String> {
    let resolve_field = |reference: &Reference| -> Outcome<Typed> {
        match fields.lookup(&policy.dataset, &Name::of(&reference.name)) {
            FieldLookup::Typed(typed) => Ok(typed.clone()),
            FieldLookup::Missing | FieldLookup::Untyped => Err(Stop::ReportedElsewhere),
        }
    };
    let resolve_attribute = |name: &str| -> Outcome<Typed> {
        attribute(name)
            .map(|declared| Typed::value(logical_type(declared.item()), false))
            .ok_or(Stop::ReportedElsewhere)
    };
    let typer = TypingScope::new(&resolve_field, Aggregates::NotAllowed)
        .with_user_attributes(&resolve_attribute);
    match type_expression(tree, &typer) {
        Ok(found) if found.logical() == Some(&LogicalType::Boolean) => None,
        Ok(found) => Some(format!(
            "it is {}, not boolean",
            describe(&found.value_type)
        )),
        Err(Stop::Problem { message, .. }) => Some(message),
        Err(Stop::ReportedElsewhere) => None,
    }
}

/// The logical type an attribute, or each item of a list attribute, is typed with. `integer`
/// is typed as a 64-bit signed integer, the type of an integer literal.
fn logical_type(scalar: ScalarType) -> LogicalType {
    match scalar {
        ScalarType::Text => LogicalType::Text,
        ScalarType::Integer => big_integer(),
        ScalarType::Decimal { precision, scale } => decimal(precision, scale),
        ScalarType::Date => LogicalType::Date,
        ScalarType::Boolean => LogicalType::Boolean,
    }
}

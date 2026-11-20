//! The row filter of one dataset for one user: `G AND R` (E4-6.2 to E4-6.4), and the check
//! that every applicable policy has its attributes (E4-6.9, SEC-3.4).

mod error;

pub use error::RowFilterError;

use menura_model::expression::{BinaryOperator, Expression, Literal};
use menura_model::internal_model::{Dataset, RoleName, RowPolicy};

use crate::context::AdmittedSecurityContext;
use crate::role::Role;
use crate::walk::children;

/// The row filter of `dataset` for the user of `context`: `G AND R` (E4-6.2), or `None` when
/// the dataset has no policy.
///
/// The policies are read in the order of `dataset.row_policies`. The policies the user is
/// exempt from are dropped (E4-6.3); `R` is the AND of the remaining policies without
/// `roles`, TRUE if none; `G` is the OR of the remaining policies that grant one of the
/// user's roles, TRUE when the dataset has no role-scoped policy or the user is exempt from
/// all of them, and the constant FALSE when none grants (E4-6.4). Conditions keep
/// `USER_ATTRIBUTE('name')`: values reach SQL only as bound parameters (SEC-3.5), so an empty
/// list stays `x IN (USER_ATTRIBUTE('a'))` (E4-6.9a).
///
/// # Errors
///
/// [`RowFilterError::MissingAttribute`] (`ESEC-02`) for the first attribute an applicable
/// policy reads that the context does not provide, in policy order and then in order of
/// appearance (E4-6.9); [`RowFilterError::UnparsedCondition`] for a policy whose condition E0
/// did not parse, which no loaded model holds.
pub fn row_filter(
    dataset: &Dataset,
    context: &AdmittedSecurityContext,
) -> Result<Option<Expression>, RowFilterError> {
    if dataset.row_policies.is_empty() {
        return Ok(None);
    }
    let user_roles = context.roles();
    let remaining: Vec<&RowPolicy> = dataset
        .row_policies
        .iter()
        .filter(|policy| !holds_any(user_roles, policy.exempt_roles.as_deref()))
        .collect();
    let applicable: Vec<&RowPolicy> = remaining
        .iter()
        .copied()
        .filter(|policy| {
            policy
                .roles
                .as_deref()
                .is_none_or(|granted| holds_any(user_roles, Some(granted)))
        })
        .collect();
    check_attributes(&applicable, context)?;
    let restrictions = conditions(applicable.iter().filter(|policy| policy.roles.is_none()))?;
    let grants = conditions(applicable.iter().filter(|policy| policy.roles.is_some()))?;
    let scoped_remain = remaining.iter().any(|policy| policy.roles.is_some());
    let grant = match combine(BinaryOperator::Or, grants) {
        Some(grant) => grant,
        None if scoped_remain => boolean(false),
        None => boolean(true),
    };
    let restriction = combine(BinaryOperator::And, restrictions).unwrap_or_else(|| boolean(true));
    Ok(Some(binary(BinaryOperator::And, grant, restriction)))
}

/// Whether the user holds one of `roles`.
fn holds_any(user_roles: &[Role], roles: Option<&[RoleName]>) -> bool {
    roles.is_some_and(|roles| {
        roles
            .iter()
            .any(|role| user_roles.iter().any(|held| held.as_str() == role.as_str()))
    })
}

/// Fails on the first attribute of an applicable policy that `context` does not provide.
fn check_attributes(
    applicable: &[&RowPolicy],
    context: &AdmittedSecurityContext,
) -> Result<(), RowFilterError> {
    for policy in applicable {
        let condition = condition(policy)?;
        if let Some(missing) = attributes(condition)
            .into_iter()
            .find(|name| !context.attributes().contains_key(*name))
        {
            return Err(RowFilterError::MissingAttribute {
                attribute: missing.to_owned(),
            });
        }
    }
    Ok(())
}

/// The conditions of `policies`, in order.
fn conditions<'policy>(
    policies: impl Iterator<Item = &'policy &'policy RowPolicy>,
) -> Result<Vec<Expression>, RowFilterError> {
    policies.map(|policy| condition(policy).cloned()).collect()
}

/// The parsed condition of `policy`.
fn condition(policy: &RowPolicy) -> Result<&Expression, RowFilterError> {
    policy
        .condition
        .as_ref()
        .ok_or_else(|| RowFilterError::UnparsedCondition {
            location: policy.location.clone(),
        })
}

/// The attribute names `expression` reads, in order of appearance.
fn attributes(expression: &Expression) -> Vec<&str> {
    let mut found = Vec::new();
    if let Expression::UserAttribute(name) = expression {
        found.push(name.as_str());
    }
    for child in children(expression) {
        found.extend(attributes(child));
    }
    found
}

/// `terms` joined by `operator`, left to right; `None` when there is none.
fn combine(operator: BinaryOperator, terms: Vec<Expression>) -> Option<Expression> {
    terms
        .into_iter()
        .reduce(|left, right| binary(operator, left, right))
}

fn binary(operator: BinaryOperator, left: Expression, right: Expression) -> Expression {
    Expression::Binary {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    }
}

fn boolean(value: bool) -> Expression {
    Expression::Literal(Literal::Boolean(value))
}

//! Shared fixtures of the tests of E2.

#![allow(dead_code, reason = "each test file uses part of the shared fixtures")]

pub mod model;
pub mod query;
pub mod sql;

use menura_e2_bind::{AuthorizedQuery, BindCode, BindError, resolve_and_authorize};
use menura_representations::query_tree::QueryTree;

/// Runs rows 3–9 on `tree` for a user holding `roles`.
pub fn run(tree: &QueryTree, roles: &[&str]) -> Result<AuthorizedQuery, BindError> {
    resolve_and_authorize(tree, &model::model(), &query::user(roles))
}

/// The code and pointer of the first error of `tree` for a user holding `roles`.
pub fn first_error(tree: &QueryTree, roles: &[&str]) -> (BindCode, String) {
    match run(tree, roles) {
        Err(BindError::Diagnostic(diagnostic)) => {
            (diagnostic.code, diagnostic.location.pointer().to_owned())
        }
        other => panic!("expected a diagnostic, got {other:?}"),
    }
}

/// The whole diagnostic of `tree` for a user holding `roles`.
pub fn diagnostic(tree: &QueryTree, roles: &[&str]) -> menura_e2_bind::BindDiagnostic {
    match run(tree, roles) {
        Err(BindError::Diagnostic(diagnostic)) => diagnostic,
        other => panic!("expected a diagnostic, got {other:?}"),
    }
}

/// Runs E2 end to end on `tree` against `model` for a user holding `roles`.
pub fn bind_query(
    tree: &QueryTree,
    model: &menura_model::internal_model::InternalModel,
    roles: &[&str],
) -> Result<menura_representations::bound_query::BoundQuery, BindError> {
    menura_e2_bind::bind(tree, model, &query::user(roles))
}

/// The diagnostic of `bind` on `tree` against `model` for a user holding `roles`.
pub fn bind_diagnostic(
    tree: &QueryTree,
    model: &menura_model::internal_model::InternalModel,
    roles: &[&str],
) -> menura_e2_bind::BindDiagnostic {
    match bind_query(tree, model, roles) {
        Err(BindError::Diagnostic(diagnostic)) => diagnostic,
        other => panic!("expected a diagnostic, got {other:?}"),
    }
}

/// The code and pointer of the first error of `bind`.
pub fn bind_error(
    tree: &QueryTree,
    model: &menura_model::internal_model::InternalModel,
    roles: &[&str],
) -> (BindCode, String) {
    let diagnostic = bind_diagnostic(tree, model, roles);
    (diagnostic.code, diagnostic.location.pointer().to_owned())
}

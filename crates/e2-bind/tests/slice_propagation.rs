//! VS-1 row-policy propagation: a home dataset that reaches, through many-to-one steps, another
//! dataset with row policies is outside the 0.3.0 slice, whatever the user's roles (E4-6.5).

#[cfg(test)]
mod support;

use menura_e2_bind::{BindCode, BindDiagnostic};
use menura_representations::query_tree::{QueryTree, QueryTreeParts};

use support::model::{add_row_policy, customers_with_a_row_policy, model_with, set_edges};
use support::query::{dimension, metric, tree};
use support::{bind_diagnostic, bind_query};

const ANALYST: &[&str] = &["analyst"];

fn assert_not_yet_supported(diagnostic: &BindDiagnostic, pointer: &str, version: &str) {
    assert_eq!(diagnostic.code, BindCode::NotYetSupported);
    assert_eq!(diagnostic.location.pointer(), pointer);
    assert!(
        diagnostic
            .message
            .ends_with(&format!("not supported before {version}")),
        "{}",
        diagnostic.message
    );
}

fn revenue_by(dimension_text: &str) -> QueryTree {
    tree(
        vec![dimension(0, dimension_text)],
        vec![metric(0, "revenue")],
        QueryTreeParts::default(),
    )
}

#[test]
fn vs_1_a_reachable_row_policy_on_a_role_the_user_lacks_is_still_0_4_0() {
    let model = model_with(customers_with_a_row_policy);
    let diagnostic = bind_diagnostic(&revenue_by("orders.region"), &model, ANALYST);
    assert_not_yet_supported(&diagnostic, "/query/dimensions/0", "0.4.0");
    assert!(
        diagnostic.message.contains("row policies"),
        "{}",
        diagnostic.message
    );
}

#[test]
fn vs_1_the_home_dataset_s_own_row_policies_are_in_the_slice() {
    let model = model_with(customers_with_a_row_policy);
    let query = tree(
        vec![dimension(0, "customers.region")],
        vec![],
        QueryTreeParts::default(),
    );
    assert!(bind_query(&query, &model, ANALYST).is_ok());
}

#[test]
fn vs_1_a_row_policy_on_the_many_side_of_the_home_dataset_is_in_the_slice() {
    let model = model_with(|parts| {
        set_edges(parts, &[("ORDERS", "CUSTOMERS")]);
        add_row_policy(parts, "ORDERS");
    });
    let query = tree(
        vec![dimension(0, "customers.region")],
        vec![],
        QueryTreeParts::default(),
    );
    assert!(bind_query(&query, &model, ANALYST).is_ok());
}

#[test]
fn vs_1_a_row_policy_two_many_to_one_steps_away_is_0_4_0() {
    let model = model_with(|parts| {
        set_edges(parts, &[("ORDERS", "CUSTOMERS"), ("CUSTOMERS", "SECRETS")]);
        add_row_policy(parts, "SECRETS");
    });
    let diagnostic = bind_diagnostic(&revenue_by("orders.region"), &model, ANALYST);
    assert_not_yet_supported(&diagnostic, "/query/dimensions/0", "0.4.0");
}

#[test]
fn vs_1_a_row_policy_reached_only_through_a_one_to_many_step_is_in_the_slice() {
    let model = model_with(|parts| {
        set_edges(parts, &[("ORDERS", "CUSTOMERS"), ("ORDERS", "SECRETS")]);
        add_row_policy(parts, "SECRETS");
    });
    let query = tree(
        vec![dimension(0, "customers.region")],
        vec![],
        QueryTreeParts::default(),
    );
    assert!(bind_query(&query, &model, ANALYST).is_ok());
}

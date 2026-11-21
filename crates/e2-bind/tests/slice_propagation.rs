//! VS-1b row-policy propagation: a home dataset that reaches, through many-to-one steps, another
//! dataset with row policies is in the 0.4.0 slice, whatever the user's roles (E4-6.5).

#[cfg(test)]
mod support;

use menura_model::internal_model::InternalModel;
use menura_representations::query_tree::{QueryTree, QueryTreeParts};

use support::bind_query;
use support::model::{add_row_policy, customers_with_a_row_policy, model_with, set_edges};
use support::query::{dimension, metric, tree};

const ANALYST: &[&str] = &["analyst"];

fn assert_in_the_slice(query: &QueryTree, model: &InternalModel) {
    let result = bind_query(query, model, ANALYST);
    assert!(result.is_ok(), "{:?}", result.err());
}

fn revenue_by(dimension_text: &str) -> QueryTree {
    tree(
        vec![dimension(0, dimension_text)],
        vec![metric(0, "revenue")],
        QueryTreeParts::default(),
    )
}

#[test]
fn vs_1b_a_reachable_row_policy_on_a_role_the_user_lacks_is_in_the_slice() {
    let model = model_with(customers_with_a_row_policy);
    assert_in_the_slice(&revenue_by("orders.region"), &model);
}

#[test]
fn vs_1b_a_row_policy_two_many_to_one_steps_away_is_in_the_slice() {
    let model = model_with(|parts| {
        set_edges(parts, &[("ORDERS", "CUSTOMERS"), ("CUSTOMERS", "SECRETS")]);
        add_row_policy(parts, "SECRETS");
    });
    assert_in_the_slice(&revenue_by("orders.region"), &model);
}

#[test]
fn vs_1b_the_home_dataset_s_own_row_policies_are_in_the_slice() {
    let model = model_with(customers_with_a_row_policy);
    let query = tree(
        vec![dimension(0, "customers.region")],
        vec![],
        QueryTreeParts::default(),
    );
    assert_in_the_slice(&query, &model);
}

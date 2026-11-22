//! The visit order of the row policies a population read needs (E4-6.9, E4-6.9b): the
//! datasets of the visible rows of the home dataset first, then the population dataset, then
//! the datasets of its own visible rows, each once.

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_e4_plan::PlanError;
use menura_model::internal_model::{InternalModel, RowPolicy};
use support::population::{active, plan, plan_with_time, subscriptions, team_policy};
use support::propagation::{linked_model, stores};
use support::{located, metrics, model_with, orders, policy, tree, user};

fn needing(attribute: &str, pointer: &str) -> RowPolicy {
    RowPolicy {
        name: format!("{attribute}_scope"),
        condition: Some(tree(&format!("region IN (USER_ATTRIBUTE('{attribute}'))"))),
        roles: None,
        exempt_roles: None,
        location: located(pointer),
    }
}

fn reading_now(pointer: &str) -> RowPolicy {
    RowPolicy {
        name: "recent".to_owned(),
        condition: Some(tree("valid_from <= CURRENT_DATE")),
        roles: None,
        exempt_roles: None,
        location: located(pointer),
    }
}

fn missing_attribute(error: &PlanError) -> String {
    assert!(matches!(error, PlanError::MissingAttribute(_)), "{error:?}");
    error.to_diagnostic().expect("a diagnostic").message
}

/// `ORDERS` with `home_policies`, and `SUBSCRIPTIONS`, which declares `ACTIVE`, with
/// `population_policies`.
fn two_datasets(
    home_policies: Vec<RowPolicy>,
    population_policies: Vec<RowPolicy>,
) -> InternalModel {
    model_with(
        vec![
            orders(home_policies),
            subscriptions(vec![active(None)], population_policies),
        ],
        metrics(),
    )
}

#[test]
fn e4_6_9_the_home_policies_are_visited_before_the_population_dataset() {
    let model = two_datasets(
        vec![policy(
            "home_scope",
            "region = USER_ATTRIBUTE('region')",
            None,
            None,
        )],
        vec![team_policy()],
    );
    let error = plan("ACTIVE", &model, &user("[]", "{}")).expect_err("a missing attribute");

    assert!(missing_attribute(&error).contains("`region`"));
}

#[test]
fn e4_6_9_the_population_dataset_is_visited_after_the_home_dataset() {
    let model = two_datasets(
        vec![policy(
            "home_scope",
            "region = USER_ATTRIBUTE('region')",
            None,
            None,
        )],
        vec![team_policy()],
    );
    let error = plan("ACTIVE", &model, &user("[]", r#"{"region": "north"}"#))
        .expect_err("a missing attribute");

    assert!(missing_attribute(&error).contains("`team`"));
}

#[test]
fn e4_6_9_the_population_dataset_is_visited_before_the_datasets_of_its_visible_rows() {
    let model = linked_model(
        vec![
            orders(Vec::new()),
            stores_needing_regions(),
            subscriptions(vec![active(None)], vec![team_policy()]),
        ],
        &[(
            "SUBSCRIPTIONS_TO_STORES",
            ("SUBSCRIPTIONS", "STORE_ID"),
            ("STORES", "STORE_ID"),
        )],
    );
    let error = plan("ACTIVE", &model, &user("[]", "{}")).expect_err("a missing attribute");

    assert!(missing_attribute(&error).contains("`team`"));
}

fn stores_needing_regions() -> menura_model::internal_model::Dataset {
    let mut dataset = stores();
    dataset.row_policies = vec![needing(
        "regions",
        "/semantic_model/0/datasets/2/custom_extensions/0/data",
    )];
    dataset
}

#[test]
fn e4_6_9_a_dataset_already_visited_from_the_home_dataset_is_visited_once() {
    let model = linked_model(
        vec![
            orders(Vec::new()),
            stores_needing_regions(),
            subscriptions(vec![active(None)], Vec::new()),
        ],
        &[
            (
                "ORDERS_TO_STORES",
                ("ORDERS", "REGION"),
                ("STORES", "REGION"),
            ),
            (
                "SUBSCRIPTIONS_TO_STORES",
                ("SUBSCRIPTIONS", "STORE_ID"),
                ("STORES", "STORE_ID"),
            ),
        ],
    );
    let node = plan("ACTIVE", &model, &user("[]", r#"{"regions": ["east"]}"#));

    assert!(node.is_ok(), "{node:?}");
}

#[test]
fn e4_6_9b_esec_02_of_the_population_dataset_comes_before_eqry_03_of_the_home_dataset() {
    let model = two_datasets(
        vec![reading_now(
            "/semantic_model/0/datasets/0/custom_extensions/0/data",
        )],
        vec![team_policy()],
    );
    let error = plan("ACTIVE", &model, &user("[]", "{}")).expect_err("a missing attribute");

    assert!(missing_attribute(&error).contains("`team`"));
}

#[test]
fn e4_6_9b_a_population_policy_reading_now_needs_reference_time() {
    let model = two_datasets(
        Vec::new(),
        vec![reading_now(
            "/semantic_model/0/datasets/1/custom_extensions/1/data",
        )],
    );
    let error = plan("ACTIVE", &model, &user("[]", "{}")).expect_err("no reference time");
    assert_eq!(error, PlanError::MissingReferenceTime);

    let node = plan_with_time("ACTIVE", &model, &user("[]", "{}"), true);
    assert!(node.is_ok(), "{node:?}");
}

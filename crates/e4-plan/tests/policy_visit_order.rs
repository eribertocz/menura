//! `ESEC-02`, then `EQRY-03`, over every dataset whose policies take part in the visible rows
//! of the home dataset, in the visit order of E4-6.9 (E4-6.9, E4-6.9b, E2-5 row 18).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_e4_plan::PlanError;
use menura_model::internal_model::{Dataset, InternalModel, RowPolicy};
use menura_representations::bound_query::BoundQueryParts;
use menura_security::AdmittedSecurityContext;
use support::propagation::{
    CUSTOMERS_TO_STORES, ORDERS_TO_CUSTOMERS, ORDERS_TO_WAREHOUSES, WAREHOUSES_TO_STORES,
    customers, home_branch, home_orders, linked_model, stores, warehouses, with_policies,
};
use support::{build_plan, metric_reference, policy, query, user};

fn needs(policy_name: &str, attribute: &str) -> RowPolicy {
    policy(
        policy_name,
        &format!("region = USER_ATTRIBUTE('{attribute}')"),
        None,
        None,
    )
}

fn reads_now(policy_name: &str) -> RowPolicy {
    policy(policy_name, "CURRENT_DATE IS NOT NULL", None, None)
}

/// `ORDERS` → `CUSTOMERS` → `STORES` and `ORDERS` → `WAREHOUSES` → `STORES`, with the given
/// policies.
fn model(
    orders: Vec<RowPolicy>,
    customer: Vec<RowPolicy>,
    store: Vec<RowPolicy>,
    warehouse: Vec<RowPolicy>,
) -> InternalModel {
    let datasets: Vec<Dataset> = vec![
        with_policies(customers(), customer),
        with_policies(home_orders(), orders),
        with_policies(stores(), store),
        with_policies(warehouses(), warehouse),
    ];
    linked_model(
        datasets,
        &[
            ORDERS_TO_CUSTOMERS,
            CUSTOMERS_TO_STORES,
            ORDERS_TO_WAREHOUSES,
            WAREHOUSES_TO_STORES,
        ],
    )
}

fn error_of(model: &InternalModel, context: &AdmittedSecurityContext) -> PlanError {
    build_plan(
        &home_branch(),
        &query(
            Vec::new(),
            vec![metric_reference("TOTAL_AMOUNT", 0)],
            BoundQueryParts::default(),
        ),
        context,
        model,
    )
    .expect_err("a compilation error")
}

fn missing_attribute(error: &PlanError) -> String {
    let diagnostic = error.to_diagnostic().expect("a diagnostic");
    assert_eq!(diagnostic.code, "ESEC-02");
    diagnostic.message
}

fn nobody() -> AdmittedSecurityContext {
    user("[]", "{}")
}

#[test]
fn e4_6_9_the_home_dataset_is_visited_first() {
    let model = model(
        vec![needs("home_scope", "team")],
        vec![needs("customer_scope", "region")],
        Vec::new(),
        Vec::new(),
    );

    let message = missing_attribute(&error_of(&model, &nobody()));

    assert!(message.contains("`team`"), "{message}");
}

#[test]
fn e4_6_9_nearer_datasets_are_visited_before_farther_ones() {
    let model = model(
        Vec::new(),
        Vec::new(),
        vec![needs("store_scope", "region")],
        vec![needs("warehouse_scope", "team")],
    );

    let message = missing_attribute(&error_of(&model, &nobody()));

    assert!(message.contains("`team`"), "{message}");
}

#[test]
fn e4_6_9_datasets_at_the_same_distance_are_visited_by_name() {
    let model = model(
        Vec::new(),
        vec![needs("customer_scope", "region")],
        Vec::new(),
        vec![needs("warehouse_scope", "team")],
    );

    let message = missing_attribute(&error_of(&model, &nobody()));

    assert!(message.contains("`region`"), "{message}");
}

#[test]
fn e4_6_9_the_message_never_names_a_policy_or_its_dataset() {
    let model = model(
        Vec::new(),
        Vec::new(),
        vec![needs("store_scope", "region")],
        Vec::new(),
    );

    let message = missing_attribute(&error_of(&model, &nobody()));

    for secret in ["store_scope", "STORES", "stores", "USER_ATTRIBUTE"] {
        assert!(!message.contains(secret), "{message}");
    }
}

#[test]
fn e4_6_9_a_policy_the_user_is_exempt_from_never_fails() {
    let exempt = policy(
        "store_scope",
        "region = USER_ATTRIBUTE('region')",
        None,
        Some(&["auditor"]),
    );
    let model = model(Vec::new(), Vec::new(), vec![exempt], Vec::new());

    let plan = build_plan(
        &home_branch(),
        &query(
            Vec::new(),
            vec![metric_reference("TOTAL_AMOUNT", 0)],
            BoundQueryParts::default(),
        ),
        &user(r#"["auditor"]"#, "{}"),
        &model,
    );

    assert!(plan.is_ok(), "{plan:?}");
}

#[test]
fn e4_6_9b_a_related_policy_reading_now_without_reference_time_is_eqry_03() {
    let model = model(
        Vec::new(),
        Vec::new(),
        vec![reads_now("store_recent")],
        Vec::new(),
    );

    let error = error_of(&model, &nobody());

    assert_eq!(error, PlanError::MissingReferenceTime);
}

#[test]
fn e2_5_row_18_every_esec_02_check_comes_before_eqry_03() {
    let model = model(
        Vec::new(),
        vec![reads_now("customer_recent")],
        vec![needs("store_scope", "region")],
        Vec::new(),
    );

    let message = missing_attribute(&error_of(&model, &nobody()));

    assert!(message.contains("`region`"), "{message}");
}

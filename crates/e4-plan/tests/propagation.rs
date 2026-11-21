//! Row-policy propagation: every `Scan` reads the visible rows of its dataset (E4-6.1, E4-6.5,
//! E4-6.5a, E4-6.5b, E4-6.5c, E4-6.6, E4-6.7, E4-6.10, E2-4.5).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_model::expression::Expression;
use menura_model::internal_model::{Dataset, InternalModel, RowPolicy};
use menura_representations::bound_query::{BoundQuery, BoundQueryParts};
use menura_representations::logical_plan::{Operation, PlanNode};
use menura_representations::paths::{Branches, JoinStep, JoinTree, QueryElement};
use menura_security::AdmittedSecurityContext;
use support::joins::{joined_branches, joined_dimension, path};
use support::plan::{column_names, kinds, node, nodes, predicate};
use support::propagation::{
    BILLED_TO, CUSTOMERS_TO_STORES, Link, ORDERS_TO_CUSTOMERS, ORDERS_TO_WAREHOUSES, SHIPPED_TO,
    STORE_OWNER, WAREHOUSES_TO_STORES, across, customers, home_branch, home_orders, linked_model,
    readable_by, stores, warehouses, with_policies,
};
use support::{build_plan, dimension, metric_reference, name, policy, query, tree, user};

/// The policy of `STORES` and `CUSTOMERS`: the user's regions, for analysts, auditors exempt.
fn region_policy() -> RowPolicy {
    policy(
        "region_scope",
        "region IN (USER_ATTRIBUTE('regions'))",
        Some(&["analyst"]),
        Some(&["auditor"]),
    )
}

/// The predicate of [`region_policy`] for an analyst, over the columns of the related row.
fn region_predicate() -> Expression {
    tree(r#""REGION" IN (USER_ATTRIBUTE('regions')) AND TRUE"#)
}

fn analyst() -> AdmittedSecurityContext {
    user(r#"["analyst"]"#, r#"{"regions": ["north"]}"#)
}

fn auditor() -> AdmittedSecurityContext {
    user(r#"["auditor"]"#, "{}")
}

fn total_query() -> BoundQuery {
    query(
        Vec::new(),
        vec![metric_reference("TOTAL_AMOUNT", 0)],
        BoundQueryParts::default(),
    )
}

fn plan_of(
    branches: &Branches,
    model: &InternalModel,
    context: &AdmittedSecurityContext,
) -> PlanNode {
    build_plan(branches, &total_query(), context, model).expect("a plan")
}

/// The chain `ORDERS` → `CUSTOMERS` → `STORES`, with `store_policies` on `STORES`.
fn chain_model(home: Dataset, store_policies: Vec<RowPolicy>, extra: &[Link]) -> InternalModel {
    let mut links = vec![ORDERS_TO_CUSTOMERS, CUSTOMERS_TO_STORES];
    links.extend_from_slice(extra);
    linked_model(
        vec![customers(), home, with_policies(stores(), store_policies)],
        &links,
    )
}

fn semi_join_parts(semi_join: &PlanNode) -> (&[JoinStep], &Expression) {
    match semi_join.operation() {
        Operation::SemiJoin {
            path, predicate, ..
        } => (path, predicate),
        _ => panic!("not a SemiJoin"),
    }
}

/// The paths and predicates of the `SemiJoin` nodes of `plan`, from the `Scan` up.
fn semi_joins(plan: &PlanNode) -> Vec<(Vec<JoinStep>, Expression)> {
    let mut found: Vec<(Vec<JoinStep>, Expression)> = nodes(plan, "SemiJoin")
        .into_iter()
        .map(|semi_join| {
            let (path, predicate) = semi_join_parts(semi_join);
            (path.to_vec(), predicate.clone())
        })
        .collect();
    found.reverse();
    found
}

#[test]
fn e4_6_5_a_two_step_chain_gives_one_semi_join_per_followed_relationship() {
    let plan = plan_of(
        &home_branch(),
        &chain_model(home_orders(), vec![region_policy()], &[]),
        &analyst(),
    );

    assert_eq!(
        kinds(&plan),
        ["Project", "Aggregate", "SemiJoin", "SemiJoin", "Scan"]
    );
    assert_eq!(
        semi_joins(&plan),
        [
            (vec![across(ORDERS_TO_CUSTOMERS)], tree("TRUE")),
            (
                vec![across(ORDERS_TO_CUSTOMERS), across(CUSTOMERS_TO_STORES)],
                region_predicate()
            ),
        ]
    );
}

#[test]
fn e4_6_5a_the_scan_reads_the_foreign_key_of_each_followed_relationship() {
    let plan = plan_of(
        &home_branch(),
        &chain_model(home_orders(), vec![region_policy()], &[]),
        &analyst(),
    );

    assert_eq!(column_names(node(&plan, "Scan")), ["AMOUNT", "CUSTOMER_ID"]);
}

#[test]
fn e4_6_1_the_own_policy_filter_is_directly_over_the_scan_and_below_the_semi_joins() {
    let home = with_policies(
        home_orders(),
        vec![policy(
            "own_region",
            "region IN (USER_ATTRIBUTE('regions'))",
            None,
            None,
        )],
    );
    let plan = plan_of(
        &home_branch(),
        &chain_model(home, vec![region_policy()], &[]),
        &analyst(),
    );

    assert_eq!(
        kinds(&plan),
        [
            "Project",
            "Aggregate",
            "SemiJoin",
            "SemiJoin",
            "Filter",
            "Scan"
        ]
    );
    assert_eq!(
        predicate(node(&plan, "Filter")),
        &tree(r#"TRUE AND "REGION" IN (USER_ATTRIBUTE('regions'))"#)
    );
}

#[test]
fn e4_6_5_a_dataset_that_reaches_no_policy_gets_no_semi_join() {
    let plan = plan_of(
        &home_branch(),
        &chain_model(home_orders(), Vec::new(), &[]),
        &analyst(),
    );

    assert_eq!(kinds(&plan), ["Project", "Aggregate", "Scan"]);
}

#[test]
fn e4_6_5a_an_exempt_user_still_needs_a_related_row() {
    let plan = plan_of(
        &home_branch(),
        &chain_model(home_orders(), vec![region_policy()], &[]),
        &auditor(),
    );

    let predicates: Vec<Expression> = semi_joins(&plan)
        .into_iter()
        .map(|(_, predicate)| predicate)
        .collect();
    assert_eq!(predicates, [tree("TRUE"), tree("TRUE AND TRUE")]);
}

#[test]
fn e4_6_4_a_user_without_grant_gets_a_false_predicate_on_the_related_row() {
    let plan = plan_of(
        &home_branch(),
        &chain_model(home_orders(), vec![region_policy()], &[]),
        &user("[]", "{}"),
    );

    assert_eq!(
        semi_joins(&plan)
            .last()
            .map(|(_, predicate)| predicate.clone()),
        Some(tree("FALSE AND TRUE"))
    );
}

#[test]
fn e4_6_5b_a_relationship_back_to_a_dataset_on_the_chain_is_not_followed() {
    let plan = plan_of(
        &home_branch(),
        &chain_model(home_orders(), vec![region_policy()], &[STORE_OWNER]),
        &analyst(),
    );

    let paths: Vec<Vec<JoinStep>> = semi_joins(&plan)
        .into_iter()
        .map(|(path, _)| path)
        .collect();
    assert_eq!(
        paths,
        [
            vec![across(ORDERS_TO_CUSTOMERS)],
            vec![across(ORDERS_TO_CUSTOMERS), across(CUSTOMERS_TO_STORES)],
        ]
    );
}

#[test]
fn e4_6_5b_a_cycle_through_the_home_dataset_is_cut_at_the_home_dataset() {
    let home = with_policies(customers(), vec![region_policy()]);
    let model = linked_model(
        vec![home, with_policies(stores(), vec![region_policy()])],
        &[CUSTOMERS_TO_STORES, STORE_OWNER],
    );
    let branches = Branches::dimension_only(name("CUSTOMERS"), JoinTree::default());
    let plan = build_plan(
        &branches,
        &query(
            vec![customers_region()],
            Vec::new(),
            BoundQueryParts::default(),
        ),
        &analyst(),
        &model,
    )
    .expect("a plan");

    assert_eq!(
        semi_joins(&plan),
        [(vec![across(CUSTOMERS_TO_STORES)], region_predicate())]
    );
}

fn customers_region() -> menura_representations::bound_query::BoundDimension {
    let mut region = dimension("REGION", None, 0);
    if let menura_representations::bound_query::BoundDimensionKind::Field { field, .. } =
        &mut region.kind
    {
        field.dataset = name("CUSTOMERS");
    }
    region
}

#[test]
fn e4_6_6_each_relationship_to_the_same_dataset_is_followed_separately() {
    let model = linked_model(
        vec![
            with_policies(customers(), vec![region_policy()]),
            home_orders(),
        ],
        &[BILLED_TO, SHIPPED_TO],
    );
    let plan = plan_of(&home_branch(), &model, &analyst());

    assert_eq!(
        semi_joins(&plan),
        [
            (vec![across(BILLED_TO)], region_predicate()),
            (vec![across(SHIPPED_TO)], region_predicate()),
        ]
    );
    assert_eq!(
        column_names(node(&plan, "Scan")),
        ["AMOUNT", "BILLED_TO", "SHIPPED_TO"]
    );
}

#[test]
fn e4_6_5_a_diamond_follows_both_routes_to_the_restricted_dataset() {
    let model = linked_model(
        vec![
            customers(),
            home_orders(),
            with_policies(stores(), vec![region_policy()]),
            warehouses(),
        ],
        &[
            ORDERS_TO_CUSTOMERS,
            CUSTOMERS_TO_STORES,
            ORDERS_TO_WAREHOUSES,
            WAREHOUSES_TO_STORES,
        ],
    );
    let plan = plan_of(&home_branch(), &model, &analyst());

    let paths: Vec<Vec<JoinStep>> = semi_joins(&plan)
        .into_iter()
        .map(|(path, _)| path)
        .collect();
    assert_eq!(
        paths,
        [
            vec![across(ORDERS_TO_CUSTOMERS)],
            vec![across(ORDERS_TO_WAREHOUSES)],
            vec![across(ORDERS_TO_CUSTOMERS), across(CUSTOMERS_TO_STORES)],
            vec![across(ORDERS_TO_WAREHOUSES), across(WAREHOUSES_TO_STORES)],
        ]
    );
}

#[test]
fn e4_6_7_the_anchor_of_a_dimension_only_branch_reads_its_visible_rows() {
    let branches = Branches::dimension_only(name("ORDERS"), JoinTree::default());
    let plan = build_plan(
        &branches,
        &query(
            vec![dimension("REGION", None, 0)],
            Vec::new(),
            BoundQueryParts::default(),
        ),
        &analyst(),
        &chain_model(home_orders(), vec![region_policy()], &[]),
    )
    .expect("a plan");

    assert_eq!(semi_joins(&plan).len(), 2);
    assert_eq!(kinds(&plan).last(), Some(&"Scan"));
}

#[test]
fn e4_6_10_propagation_adds_no_output_column() {
    let propagated = plan_of(
        &home_branch(),
        &chain_model(home_orders(), vec![region_policy()], &[]),
        &analyst(),
    );
    let plain = plan_of(
        &home_branch(),
        &chain_model(home_orders(), Vec::new(), &[]),
        &analyst(),
    );

    assert_eq!(propagated.schema(), plain.schema());
    for semi_join in nodes(&propagated, "SemiJoin") {
        assert_eq!(
            semi_join.schema(),
            semi_join.inputs().first().expect("an input").schema()
        );
    }
}

#[test]
fn e2_4_5_a_dataset_joined_only_for_a_policy_needs_no_authorization() {
    let model = linked_model(
        vec![
            readable_by(customers(), &["manager"]),
            home_orders(),
            readable_by(with_policies(stores(), vec![region_policy()]), &["manager"]),
        ],
        &[ORDERS_TO_CUSTOMERS, CUSTOMERS_TO_STORES],
    );
    let plan = plan_of(&home_branch(), &model, &analyst());

    assert_eq!(semi_joins(&plan).len(), 2);
}

/// `TOTAL_AMOUNT` by `CUSTOMERS.TEAM`, through one join, with policies on `CUSTOMERS` and
/// `STORES`.
fn joined_team_plan() -> PlanNode {
    let model = linked_model(
        vec![
            with_policies(customers(), vec![region_policy()]),
            home_orders(),
            with_policies(stores(), vec![region_policy()]),
        ],
        &[ORDERS_TO_CUSTOMERS, CUSTOMERS_TO_STORES],
    );
    let branches = joined_branches(
        1,
        vec![path(
            QueryElement::Dimension(0),
            vec![across(ORDERS_TO_CUSTOMERS)],
        )],
    );
    build_plan(
        &branches,
        &query(
            vec![joined_dimension("CUSTOMERS", "TEAM", None, 0)],
            vec![metric_reference("TOTAL_AMOUNT", 0)],
            BoundQueryParts::default(),
        ),
        &analyst(),
        &model,
    )
    .expect("a plan")
}

#[test]
fn e4_6_5c_a_joined_scan_reads_the_visible_rows_of_its_dataset() {
    let plan = joined_team_plan();

    let Operation::Join { right, .. } = node(&plan, "Join").operation() else {
        panic!("not a Join");
    };
    assert_eq!(kinds(right), ["SemiJoin", "Filter", "Scan"]);
    assert_eq!(
        semi_joins(right),
        [(vec![across(CUSTOMERS_TO_STORES)], region_predicate())]
    );
    assert_eq!(
        predicate(node(right, "Filter")),
        &tree(r#""ORDERS_TO_CUSTOMERS.REGION" IN (USER_ATTRIBUTE('regions')) AND TRUE"#)
    );
    assert_eq!(
        column_names(node(right, "Scan")),
        [
            "ORDERS_TO_CUSTOMERS.CUSTOMER_ID",
            "ORDERS_TO_CUSTOMERS.REGION",
            "ORDERS_TO_CUSTOMERS.STORE_ID",
            "ORDERS_TO_CUSTOMERS.TEAM",
        ]
    );
    assert_eq!(
        semi_joins(&plan),
        [
            (vec![across(ORDERS_TO_CUSTOMERS)], region_predicate()),
            (
                vec![across(ORDERS_TO_CUSTOMERS), across(CUSTOMERS_TO_STORES)],
                region_predicate()
            ),
        ]
    );
}

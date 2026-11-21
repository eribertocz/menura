//! Row-policy propagation: every `Scan` reads the visible rows of its dataset (E4-6.1, E4-6.5,
//! E4-6.5a, E4-6.5b, E4-6.5c, E4-6.6, E4-6.7, E4-6.10, E2-4.5).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_model::expression::Expression;
use menura_model::internal_model::{Dataset, InternalModel, RowPolicy};
use menura_representations::bound_query::{BoundQuery, BoundQueryParts};
use menura_representations::logical_plan::{JoinCondition, Operation, PlanNode};
use menura_representations::names::ColumnName;
use menura_representations::paths::{Branches, JoinTree, QueryElement};
use menura_security::AdmittedSecurityContext;
use support::joins::{joined_branches, joined_dimension, path};
use support::plan::{column_names, kinds, node, nodes, predicate};
use support::propagation::{
    BILLED_TO, CUSTOMERS_TO_STORES, Link, ORDERS_TO_CUSTOMERS, ORDERS_TO_WAREHOUSES, SHIPPED_TO,
    STORE_OWNER, WAREHOUSES_TO_STORES, across, customers, filter_predicates, home_branch,
    home_orders, linked_model, outline, readable_by, semi_join_relationships, stores, warehouses,
    with_policies,
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

/// The outline of the visible rows of `ORDERS` in the chain model, for an analyst.
const CHAIN_OUTLINE: &str = concat!(
    "SemiJoin ORDERS_TO_CUSTOMERS on CUSTOMER_ID=CUSTOMER_ID ",
    "all(Scan CUSTOMERS [CUSTOMER_ID]) ",
    "visible(SemiJoin CUSTOMERS_TO_STORES on STORE_ID=STORE_ID ",
    "all(Scan STORES [STORE_ID]) ",
    "visible(Filter(Scan STORES [REGION, STORE_ID])) ",
    "over(Scan CUSTOMERS [CUSTOMER_ID, STORE_ID])) ",
    "over(Scan ORDERS [AMOUNT, CUSTOMER_ID])"
);

fn chain_plan(context: &AdmittedSecurityContext, extra: &[Link]) -> PlanNode {
    plan_of(
        &home_branch(),
        &chain_model(home_orders(), vec![region_policy()], extra),
        context,
    )
}

#[test]
fn e4_6_5_a_two_step_chain_nests_the_visible_rows_of_each_related_dataset() {
    let plan = chain_plan(&analyst(), &[]);

    assert_eq!(kinds(&plan), ["Project", "Aggregate", "SemiJoin", "Scan"]);
    assert_eq!(outline(node(&plan, "SemiJoin")), CHAIN_OUTLINE);
    assert_eq!(filter_predicates(&plan), [region_predicate()]);
}

#[test]
fn e4_6_5a_the_keys_pair_the_input_columns_with_the_related_columns() {
    let plan = chain_plan(&analyst(), &[]);

    let Operation::SemiJoin {
        relationship, on, ..
    } = node(&plan, "SemiJoin").operation()
    else {
        panic!("not a SemiJoin");
    };
    assert_eq!(relationship, &name("ORDERS_TO_CUSTOMERS"));
    assert_eq!(
        on,
        &[JoinCondition {
            left: ColumnName::new("CUSTOMER_ID"),
            right: ColumnName::new("CUSTOMER_ID"),
        }]
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
        ["Project", "Aggregate", "SemiJoin", "Filter", "Scan"]
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
    let plan = chain_plan(&auditor(), &[]);

    assert_eq!(semi_join_relationships(&plan), ["ORDERS_TO_CUSTOMERS"]);
    assert_eq!(filter_predicates(&plan), [tree("TRUE AND TRUE")]);
}

#[test]
fn e4_6_4_a_user_without_grant_sees_no_related_row() {
    let plan = chain_plan(&user("[]", "{}"), &[]);

    assert_eq!(filter_predicates(&plan), [tree("FALSE AND TRUE")]);
}

#[test]
fn e4_6_5b_a_relationship_back_to_a_dataset_on_the_chain_is_not_followed() {
    let plan = chain_plan(&analyst(), &[STORE_OWNER]);

    assert_eq!(outline(node(&plan, "SemiJoin")), CHAIN_OUTLINE);
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
        outline(node(&plan, "SemiJoin")),
        concat!(
            "SemiJoin CUSTOMERS_TO_STORES on STORE_ID=STORE_ID ",
            "all(Scan STORES [STORE_ID]) ",
            "visible(Filter(Scan STORES [REGION, STORE_ID])) ",
            "over(Filter(Scan CUSTOMERS [REGION, STORE_ID]))"
        )
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

    assert_eq!(semi_join_relationships(&plan), ["BILLED_TO", "SHIPPED_TO"]);
    assert_eq!(
        filter_predicates(&plan),
        [region_predicate(), region_predicate()]
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

    assert_eq!(
        semi_join_relationships(&plan),
        ["ORDERS_TO_CUSTOMERS", "ORDERS_TO_WAREHOUSES"]
    );
    assert_eq!(
        filter_predicates(&plan),
        [region_predicate(), region_predicate()]
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

    assert_eq!(semi_join_relationships(&plan), ["ORDERS_TO_CUSTOMERS"]);
    assert_eq!(filter_predicates(&plan), [region_predicate()]);
}

#[test]
fn e4_6_10_propagation_adds_no_output_column() {
    let propagated = chain_plan(&analyst(), &[]);
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

    assert_eq!(outline(node(&plan, "SemiJoin")), CHAIN_OUTLINE);
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
    assert_eq!(
        outline(right),
        concat!(
            "SemiJoin CUSTOMERS_TO_STORES on ORDERS_TO_CUSTOMERS.STORE_ID=STORE_ID ",
            "all(Scan STORES [STORE_ID]) ",
            "visible(Filter(Scan STORES [REGION, STORE_ID])) ",
            "over(Filter(Scan CUSTOMERS [ORDERS_TO_CUSTOMERS.CUSTOMER_ID, ",
            "ORDERS_TO_CUSTOMERS.REGION, ORDERS_TO_CUSTOMERS.STORE_ID, ORDERS_TO_CUSTOMERS.TEAM]))"
        )
    );
    assert_eq!(
        predicate(node(right, "Filter")),
        &tree(r#""ORDERS_TO_CUSTOMERS.REGION" IN (USER_ATTRIBUTE('regions')) AND TRUE"#)
    );
    assert_eq!(semi_join_relationships(&plan), ["ORDERS_TO_CUSTOMERS"]);
}

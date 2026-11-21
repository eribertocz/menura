//! Joins along the join tree of a multi-dataset branch (E4-3.2, E4-3.11, E4-4.0c, E4-4.0d,
//! E3-4.2a).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_contracts::types::common::LogicalType;
use menura_e4_plan::{InvariantViolation, PlanError};
use menura_expressions::typing::{big_integer, decimal};
use menura_model::expression::{BinaryOperator, Expression};
use menura_representations::bound_query::BoundQueryParts;
use menura_representations::logical_plan::{JoinKind, PlanNode};
use menura_representations::origin::Origin;
use menura_representations::paths::{ElementPath, QueryElement};
use support::join_plan::{
    IDENTITY, aggregate_keys, condition, identity_type, join_parts, plan, scan_dataset,
    segment_plan,
};
use support::joins::{
    joined_branches, joined_dimension, joined_filter, ordered_on, path, relationship_pointer,
    sales_model, sales_model_with, sales_orders, shipped_on, to_customers, to_regions,
};
use support::plan::{column, column_names, column_reference, kinds, node, predicate};
use support::{anyone, build_plan, located, metric_reference, policy, query};

#[test]
fn e4_4_0c_e4_4_0g_one_join_reads_the_joined_dataset_through_its_instance() {
    let plan = segment_plan();

    assert_eq!(
        kinds(&plan),
        ["Project", "Aggregate", "Deduplicate", "Join", "Scan"]
    );
    let join = node(&plan, "Join");
    let (home, joined, on, prefix, kind) = join_parts(join);
    assert_eq!(kind, JoinKind::Left);
    assert_eq!(prefix, "ORDERS_TO_CUSTOMERS.");
    assert_eq!(scan_dataset(home), "ORDERS");
    assert_eq!(scan_dataset(joined), "CUSTOMERS");
    assert_eq!(
        on,
        [condition("CUSTOMER_ID", "ORDERS_TO_CUSTOMERS.CUSTOMER_ID")]
    );
    assert_eq!(
        column_names(joined),
        [
            "ORDERS_TO_CUSTOMERS.CUSTOMER_ID",
            "ORDERS_TO_CUSTOMERS.SEGMENT"
        ]
    );
    assert_eq!(
        aggregate_keys(&plan),
        [column_reference("ORDERS_TO_CUSTOMERS.SEGMENT")]
    );
}

#[test]
fn e4_3_2_a_join_outputs_left_then_right_and_the_right_columns_can_be_null() {
    let plan = segment_plan();
    let join = node(&plan, "Join");

    assert_eq!(
        join.schema().columns(),
        [
            column("AMOUNT", decimal(12, 2), true),
            column("CUSTOMER_ID", big_integer(), false),
            column(IDENTITY, identity_type(), false),
            column("ORDERS_TO_CUSTOMERS.CUSTOMER_ID", big_integer(), true),
            column("ORDERS_TO_CUSTOMERS.SEGMENT", LogicalType::Text, true),
        ]
    );
}

#[test]
fn e4_3_11_a_join_and_its_scans_come_from_the_model() {
    let plan = segment_plan();
    let join = node(&plan, "Join");
    let (home, joined, ..) = join_parts(join);

    assert_eq!(
        join.origin(),
        &Origin::Model(located(&relationship_pointer(0)))
    );
    assert_eq!(
        home.origin(),
        &Origin::Model(located("/semantic_model/0/datasets/0"))
    );
    assert_eq!(
        joined.origin(),
        &Origin::Model(located("/semantic_model/0/datasets/1"))
    );
    assert!(matches!(
        node(&plan, "Deduplicate").origin(),
        Origin::Model(_)
    ));
}

fn region_plan() -> PlanNode {
    plan(
        &joined_branches(
            1,
            vec![path(
                QueryElement::Dimension(0),
                vec![to_customers(), to_regions()],
            )],
        ),
        vec![joined_dimension("REGIONS", "NAME", None, 0)],
        1,
        BoundQueryParts::default(),
    )
}

#[test]
fn e4_4_0c_a_chain_of_joins_follows_path_order() {
    let plan = region_plan();

    assert_eq!(
        kinds(&plan),
        [
            "Project",
            "Aggregate",
            "Deduplicate",
            "Join",
            "Join",
            "Scan"
        ]
    );
    let (inner, regions, on, prefix, _) = join_parts(node(&plan, "Join"));
    assert_eq!(prefix, "ORDERS_TO_CUSTOMERS.CUSTOMERS_TO_REGIONS.");
    assert_eq!(scan_dataset(regions), "REGIONS");
    assert_eq!(
        on,
        [condition(
            "ORDERS_TO_CUSTOMERS.REGION_ID",
            "ORDERS_TO_CUSTOMERS.CUSTOMERS_TO_REGIONS.REGION_ID"
        )]
    );
    assert_eq!(
        aggregate_keys(&plan),
        [column_reference(
            "ORDERS_TO_CUSTOMERS.CUSTOMERS_TO_REGIONS.NAME"
        )]
    );
    assert_eq!(scan_dataset(join_parts(inner).1), "CUSTOMERS");
}

#[test]
fn e4_4_0c_an_intermediate_instance_reads_the_keys_of_both_its_joins() {
    let plan = region_plan();
    let (inner, ..) = join_parts(node(&plan, "Join"));
    let (_, customers, on, prefix, _) = join_parts(inner);

    assert_eq!(prefix, "ORDERS_TO_CUSTOMERS.");
    assert_eq!(
        column_names(customers),
        [
            "ORDERS_TO_CUSTOMERS.CUSTOMER_ID",
            "ORDERS_TO_CUSTOMERS.REGION_ID"
        ]
    );
    assert_eq!(
        on,
        [condition("CUSTOMER_ID", "ORDERS_TO_CUSTOMERS.CUSTOMER_ID")]
    );
}

#[test]
fn e3_4_2a_two_role_playing_instances_are_two_scans() {
    let plan = plan(
        &joined_branches(
            1,
            vec![
                path(QueryElement::Dimension(0), vec![ordered_on()]),
                path(QueryElement::Dimension(1), vec![shipped_on()]),
            ],
        ),
        vec![
            joined_dimension("CALENDAR", "MONTH_NAME", None, 0),
            joined_dimension("CALENDAR", "MONTH_NAME", None, 1),
        ],
        1,
        BoundQueryParts::default(),
    );

    let outer = node(&plan, "Join");
    let (inner, shipped, outer_on, outer_prefix, _) = join_parts(outer);
    let (_, ordered, inner_on, inner_prefix, _) = join_parts(inner);
    assert_eq!(inner_prefix, "ORDERED_ON.");
    assert_eq!(outer_prefix, "SHIPPED_ON.");
    assert_eq!(scan_dataset(ordered), "CALENDAR");
    assert_eq!(scan_dataset(shipped), "CALENDAR");
    assert_eq!(inner_on, [condition("ORDER_DATE", "ORDERED_ON.DAY")]);
    assert_eq!(outer_on, [condition("SHIP_DATE", "SHIPPED_ON.DAY")]);
    assert_eq!(
        aggregate_keys(&plan),
        [
            column_reference("ORDERED_ON.MONTH_NAME"),
            column_reference("SHIPPED_ON.MONTH_NAME"),
        ]
    );
}

#[test]
fn e4_3_3_e4_4_0d_a_filter_over_two_datasets_reads_each_through_its_instance() {
    let parts = BoundQueryParts {
        filters: vec![joined_filter(
            "orders.status = customers.segment",
            &["CUSTOMERS", "ORDERS"],
            0,
        )],
        ..BoundQueryParts::default()
    };
    let paths: Vec<ElementPath> = vec![
        path(
            QueryElement::Filter {
                filter: 0,
                dataset: support::name("CUSTOMERS"),
            },
            vec![to_customers()],
        ),
        path(
            QueryElement::Filter {
                filter: 0,
                dataset: support::name("ORDERS"),
            },
            Vec::new(),
        ),
    ];
    let plan = plan(&joined_branches(1, paths), Vec::new(), 1, parts);

    assert_eq!(
        predicate(node(&plan, "Filter")),
        &Expression::Binary {
            operator: BinaryOperator::Equal,
            left: Box::new(column_reference("STATUS")),
            right: Box::new(column_reference("ORDERS_TO_CUSTOMERS.SEGMENT")),
        }
    );
}

#[test]
fn e3_4_2a_an_element_bound_to_an_instance_of_another_dataset_is_an_internal_error() {
    let result = build_plan(
        &joined_branches(
            1,
            vec![path(QueryElement::Dimension(0), vec![ordered_on()])],
        ),
        &query(
            vec![joined_dimension("CUSTOMERS", "SEGMENT", None, 0)],
            vec![metric_reference("TOTAL_AMOUNT", 0)],
            BoundQueryParts::default(),
        ),
        &anyone(),
        &sales_model(),
    );

    assert!(matches!(
        result,
        Err(PlanError::Invariant {
            violation: InvariantViolation::ForeignField,
            ..
        })
    ));
}

#[test]
fn e4_3_1_e4_4_0a_the_plan_scans_the_home_dataset_first() {
    let plan = segment_plan();
    let (home, joined, ..) = join_parts(node(&plan, "Join"));

    assert_eq!(scan_dataset(home), "ORDERS");
    assert!(home.inputs().is_empty());
    assert_eq!(scan_dataset(joined), "CUSTOMERS");
    assert!(joined.inputs().is_empty());
}

#[test]
fn e4_4_0b_the_home_row_policy_filters_the_home_scan_before_the_joins() {
    let mut home = sales_orders();
    home.row_policies = vec![policy("open_orders", "orders.status = 'open'", None, None)];
    let plan = build_plan(
        &joined_branches(
            1,
            vec![path(QueryElement::Dimension(0), vec![to_customers()])],
        ),
        &query(
            vec![joined_dimension("CUSTOMERS", "SEGMENT", None, 0)],
            vec![metric_reference("TOTAL_AMOUNT", 0)],
            BoundQueryParts::default(),
        ),
        &anyone(),
        &sales_model_with(home, Vec::new()),
    )
    .expect("a plan");

    assert_eq!(
        kinds(&plan),
        [
            "Project",
            "Aggregate",
            "Deduplicate",
            "Join",
            "Filter",
            "Scan"
        ]
    );
    let (policy_filter, ..) = join_parts(node(&plan, "Join"));
    assert_eq!(
        policy_filter.origin(),
        &Origin::Model(located(support::POLICY_POINTER))
    );
    let home = policy_filter.inputs().first().copied().expect("one input");
    assert_eq!(scan_dataset(home), "ORDERS");
}

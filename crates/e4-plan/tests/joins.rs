//! Joins along the join tree and `Deduplicate` before the `Aggregate` of a multi-dataset
//! branch (E4-3, E4-4, E3-4.2a, E0E-3.3, E0G-1.2, E0G-1.3).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_contracts::types::common::{Grain, IntegerBits, LogicalType};
use menura_e4_plan::{InvariantViolation, PlanError};
use menura_expressions::typing::{big_integer, decimal};
use menura_model::expression::{
    AggregateCall, BinaryOperator, DateUnit, Expression, SimpleAggregate,
};
use menura_representations::bound_query::{BoundDimension, BoundQueryParts};
use menura_representations::logical_plan::{JoinCondition, JoinKind, Operation, PlanNode};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;
use menura_representations::paths::{Branches, ElementPath, QueryElement};
use support::joins::{
    joined_branches, joined_dimension, joined_filter, ordered_on, path, relationship_pointer,
    sales_model, sales_model_with, sales_orders, shipped_on, to_customers, to_regions,
};
use support::plan::{column, column_names, column_reference, kinds, node, predicate};
use support::{anyone, build_plan, located, metric_reference, query, tree};

const IDENTITY: &str = "menura.row_identity";

fn plan(
    branches: &Branches,
    dimensions: Vec<BoundDimension>,
    metric_count: usize,
    parts: BoundQueryParts,
) -> PlanNode {
    let metrics = (0..metric_count)
        .map(|index| metric_reference("TOTAL_AMOUNT", index))
        .collect();
    build_plan(
        branches,
        &query(dimensions, metrics, parts),
        &anyone(),
        &sales_model(),
    )
    .expect("a plan")
}

fn segment_plan() -> PlanNode {
    plan(
        &joined_branches(
            1,
            vec![path(QueryElement::Dimension(0), vec![to_customers()])],
        ),
        vec![joined_dimension("CUSTOMERS", "SEGMENT", None, 0)],
        1,
        BoundQueryParts::default(),
    )
}

fn join_parts(join: &PlanNode) -> (&PlanNode, &PlanNode, &[JoinCondition], &str, JoinKind) {
    match join.operation() {
        Operation::Join {
            left,
            right,
            on,
            right_prefix,
            kind,
            ..
        } => (left, right, on, right_prefix.as_str(), *kind),
        _ => panic!("not a Join"),
    }
}

fn condition(left: &str, right: &str) -> JoinCondition {
    JoinCondition {
        left: ColumnName::new(left),
        right: ColumnName::new(right),
    }
}

fn aggregate_keys(plan: &PlanNode) -> Vec<Expression> {
    match node(plan, "Aggregate").operation() {
        Operation::Aggregate { keys, .. } => {
            keys.iter().map(|key| key.expression.clone()).collect()
        }
        _ => panic!("not an Aggregate"),
    }
}

fn deduplicate_parts(plan: &PlanNode) -> (Vec<Expression>, Vec<&str>) {
    match node(plan, "Deduplicate").operation() {
        Operation::Deduplicate {
            group, identity, ..
        } => (
            group.clone(),
            identity.iter().map(ColumnName::as_str).collect(),
        ),
        _ => panic!("not a Deduplicate"),
    }
}

fn identity_type() -> LogicalType {
    LogicalType::Integer {
        bits: IntegerBits::SixtyFour,
        signed: true,
    }
}

fn scan_dataset(scan: &PlanNode) -> &str {
    match scan.operation() {
        Operation::Scan { dataset } => dataset.as_str(),
        _ => panic!("not a Scan"),
    }
}

#[test]
fn e4_4_0c_one_join_reads_the_joined_dataset_through_its_instance() {
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

#[test]
fn e4_4_0c_a_chain_of_joins_follows_path_order() {
    let plan = plan(
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
    );

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
    let outer = node(&plan, "Join");
    let (inner, regions, on, prefix, _) = join_parts(outer);
    assert_eq!(prefix, "ORDERS_TO_CUSTOMERS.CUSTOMERS_TO_REGIONS.");
    assert_eq!(scan_dataset(regions), "REGIONS");
    assert_eq!(
        on,
        [condition(
            "ORDERS_TO_CUSTOMERS.REGION_ID",
            "ORDERS_TO_CUSTOMERS.CUSTOMERS_TO_REGIONS.REGION_ID"
        )]
    );
    let (_, customers, inner_on, inner_prefix, _) = join_parts(inner);
    assert_eq!(inner_prefix, "ORDERS_TO_CUSTOMERS.");
    assert_eq!(
        column_names(customers),
        [
            "ORDERS_TO_CUSTOMERS.CUSTOMER_ID",
            "ORDERS_TO_CUSTOMERS.REGION_ID"
        ]
    );
    assert_eq!(
        inner_on,
        [condition("CUSTOMER_ID", "ORDERS_TO_CUSTOMERS.CUSTOMER_ID")]
    );
    assert_eq!(
        aggregate_keys(&plan),
        [column_reference(
            "ORDERS_TO_CUSTOMERS.CUSTOMERS_TO_REGIONS.NAME"
        )]
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
fn e4_4_0f_fan_out_at_a_month_grain_counts_a_home_row_once_per_group() {
    let plan = plan(
        &joined_branches(
            1,
            vec![path(QueryElement::Dimension(0), vec![ordered_on()])],
        ),
        vec![joined_dimension("CALENDAR", "DAY", Some(Grain::Month), 0)],
        1,
        BoundQueryParts::default(),
    );

    let month = Expression::DateTrunc {
        unit: DateUnit::Month,
        operand: Box::new(column_reference("ORDERED_ON.DAY")),
    };
    let (group, identity) = deduplicate_parts(&plan);
    assert_eq!(group, [month.clone()]);
    assert_eq!(identity, [IDENTITY]);
    assert_eq!(aggregate_keys(&plan), [month]);
    let deduplicate = node(&plan, "Deduplicate");
    assert_eq!(deduplicate.schema(), node(&plan, "Join").schema());
    assert!(!plan.is_simple_plan());
}

#[test]
fn e4_4_0f_the_identity_is_synthesized_over_the_home_scan() {
    let plan = segment_plan();
    let (home, ..) = join_parts(node(&plan, "Join"));

    assert_eq!(column_names(home), ["AMOUNT", "CUSTOMER_ID", IDENTITY]);
    assert_eq!(
        home.schema().columns().last(),
        Some(&column(IDENTITY, identity_type(), false))
    );
}

#[test]
fn e4_4_0f_the_identity_never_takes_the_name_of_a_field() {
    let mut home = sales_orders();
    let mut clash = home.fields.first().cloned().expect("a field");
    clash.name = support::name(IDENTITY);
    home.fields.push(clash);
    let model = sales_model_with(home, Vec::new());
    let result = build_plan(
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
        &model,
    )
    .expect("a plan");

    let (_, identity) = deduplicate_parts(&result);
    assert_eq!(identity, ["menura.row_identity."]);
}

#[test]
fn e4_4_0e_a_join_for_a_filter_alone_still_gets_a_deduplicate() {
    let parts = BoundQueryParts {
        filters: vec![joined_filter(
            "customers.segment = 'retail'",
            &["CUSTOMERS"],
            0,
        )],
        ..BoundQueryParts::default()
    };
    let element = QueryElement::Filter {
        filter: 0,
        dataset: support::name("CUSTOMERS"),
    };
    let plan = plan(
        &joined_branches(1, vec![path(element, vec![to_customers()])]),
        Vec::new(),
        1,
        parts,
    );

    assert_eq!(
        kinds(&plan),
        [
            "Project",
            "Aggregate",
            "Deduplicate",
            "Filter",
            "Join",
            "Scan"
        ]
    );
    assert_eq!(
        predicate(node(&plan, "Filter")),
        &Expression::Binary {
            operator: BinaryOperator::Equal,
            left: Box::new(column_reference("ORDERS_TO_CUSTOMERS.SEGMENT")),
            right: Box::new(tree("'retail'")),
        }
    );
    let (group, identity) = deduplicate_parts(&plan);
    assert!(group.is_empty());
    assert_eq!(identity, [IDENTITY]);
}

#[test]
fn e4_4_0d_a_filter_over_two_datasets_reads_each_through_its_instance() {
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
fn e4_4_0e_a_dimension_only_branch_never_gets_a_deduplicate() {
    let plan = plan(
        &joined_branches(
            0,
            vec![path(QueryElement::Dimension(0), vec![to_customers()])],
        ),
        vec![joined_dimension("CUSTOMERS", "SEGMENT", None, 0)],
        0,
        BoundQueryParts::default(),
    );

    assert_eq!(kinds(&plan), ["Project", "Aggregate", "Join", "Scan"]);
    let (home, ..) = join_parts(node(&plan, "Join"));
    assert_eq!(column_names(home), ["CUSTOMER_ID"]);
    assert!(plan.is_simple_plan());
}

#[test]
fn e0g_1_2_a_component_reads_the_home_rows_whatever_the_joins() {
    let plan = segment_plan();
    let Operation::Aggregate { components, .. } = node(&plan, "Aggregate").operation() else {
        panic!("not an Aggregate");
    };
    let component = components.first().expect("one component");

    assert_eq!(
        component.call,
        AggregateCall::Simple {
            function: SimpleAggregate::Sum,
            argument: Box::new(column_reference("AMOUNT")),
        }
    );
}

#[test]
fn e0e_3_3_e0g_1_3_every_joined_metric_branch_is_deduplicated_by_home_row() {
    for paths in [
        vec![path(QueryElement::Dimension(0), vec![to_customers()])],
        vec![path(
            QueryElement::Dimension(0),
            vec![to_customers(), to_regions()],
        )],
    ] {
        let dataset = if paths.iter().any(|path| path.steps.len() == 2) {
            ("REGIONS", "NAME")
        } else {
            ("CUSTOMERS", "SEGMENT")
        };
        let plan = plan(
            &joined_branches(1, paths),
            vec![joined_dimension(dataset.0, dataset.1, None, 0)],
            1,
            BoundQueryParts::default(),
        );
        let (_, identity) = deduplicate_parts(&plan);
        assert_eq!(identity, [IDENTITY]);
    }
}

#[test]
fn e4_4_1_unmatched_home_rows_fall_into_the_null_group() {
    let plan = segment_plan();
    let Operation::Aggregate { input, .. } = node(&plan, "Aggregate").operation() else {
        panic!("not an Aggregate");
    };
    let segment = input
        .schema()
        .columns()
        .iter()
        .find(|column| column.name.as_str() == "ORDERS_TO_CUSTOMERS.SEGMENT")
        .expect("the segment column");

    assert!(segment.nullable);
    assert!(
        node(&plan, "Aggregate")
            .schema()
            .columns()
            .first()
            .expect("the key")
            .nullable
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

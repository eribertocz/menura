//! `Deduplicate` before the `Aggregate` of a multi-dataset branch (E4-3.5, E4-4.0e, E4-4.0f,
//! E4-4.1, E4-4.2, E0E-3.3, E0G-1.2, E0G-1.3).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_contracts::types::common::Grain;
use menura_model::expression::{
    AggregateCall, BinaryOperator, DateUnit, Expression, SimpleAggregate,
};
use menura_representations::bound_query::BoundQueryParts;
use menura_representations::logical_plan::Operation;
use menura_representations::paths::QueryElement;
use support::join_plan::{
    IDENTITY, aggregate_keys, deduplicate_parts, identity_type, join_parts, plan, segment_plan,
};
use support::joins::{
    joined_branches, joined_dimension, joined_filter, ordered_on, path, sales_model_with,
    sales_orders, to_customers, to_regions,
};
use support::plan::{column, column_names, column_reference, kinds, node, predicate};
use support::{anyone, build_plan, metric_reference, query, tree};

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
    assert_eq!(group, std::slice::from_ref(&month));
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

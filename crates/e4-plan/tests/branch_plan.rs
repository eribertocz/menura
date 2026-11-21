//! The plan of the single branch of the 0.3.0 slice and its output (E4-1 to E4-4, E4-9).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_contracts::types::common::{Grain, LogicalType};
use menura_contracts::types::query::{Direction, NullsOrder};
use menura_expressions::typing::big_integer;
use menura_representations::bound_query::BoundQueryParts;
use menura_representations::logical_plan::{
    AggregateComponent, NamedExpression, Operation, PlanNode,
};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;
use support::plan::{column, column_names, kinds, node, predicate};
use support::{
    DATASET_POINTER, anyone, at, branches, build_plan, date, date_range, dimension, filter,
    located, metric_reference, model, order, query, tree,
};

fn sales_query() -> menura_representations::bound_query::BoundQuery {
    query(
        vec![
            dimension("REGION", None, 0),
            dimension("ORDER_DATE", Some(Grain::Month), 1),
        ],
        vec![
            metric_reference("AVERAGE_AMOUNT", 0),
            metric_reference("ORDER_COUNT", 1),
        ],
        BoundQueryParts {
            filters: vec![filter("orders.status = 'paid'", 0)],
            date_range: Some(date_range(
                "ORDER_DATE",
                date(2026, 1, 1),
                date(2026, 3, 31),
            )),
            order: vec![order(3, Direction::Desc, NullsOrder::Last, 0)],
            limit: None,
            ..BoundQueryParts::default()
        },
    )
}

fn sales_plan() -> PlanNode {
    build_plan(
        &branches(&[2, 1]),
        &sales_query(),
        &anyone(),
        &model(Vec::new()),
    )
    .expect("a plan")
}

fn aggregate_parts(plan: &PlanNode) -> (&[NamedExpression], &[AggregateComponent]) {
    match node(plan, "Aggregate").operation() {
        Operation::Aggregate {
            keys, components, ..
        } => (keys, components),
        _ => unreachable!(),
    }
}

fn named(name: &str, expression: &str) -> NamedExpression {
    NamedExpression {
        name: ColumnName::new(name),
        expression: tree(expression),
    }
}

#[test]
fn e4_1_e4_2_the_branch_plan_is_scan_filter_aggregate_project_sort() {
    assert_eq!(
        kinds(&sales_plan()),
        ["Sort", "Project", "Aggregate", "Filter", "Scan"]
    );
}

#[test]
fn e4_4_step_1_the_scan_reads_the_home_dataset_and_projects_the_fields_it_needs() {
    let plan = sales_plan();
    let scan = node(&plan, "Scan");
    assert!(matches!(
        scan.operation(),
        Operation::Scan { dataset, row_identity: None } if dataset.as_str() == "ORDERS"
    ));
    assert_eq!(
        column_names(scan),
        ["AMOUNT", "ORDER_DATE", "REGION", "STATUS"]
    );
    assert_eq!(
        scan.schema().columns().get(2),
        Some(&column("REGION", LogicalType::Text, true))
    );
    assert_eq!(scan.origin(), &Origin::Model(located(DATASET_POINTER)));
}

#[test]
fn e4_4_step_4_the_query_filters_are_one_conjunction_followed_by_the_date_range() {
    let plan = sales_plan();
    let filter = node(&plan, "Filter");
    assert_eq!(
        predicate(filter),
        &tree(
            r#""STATUS" = 'paid' AND ("ORDER_DATE" >= DATE '2026-01-01' AND "ORDER_DATE" <= DATE '2026-03-31')"#
        )
    );
    assert_eq!(filter.origin(), &Origin::Query(at("/query/filters/0")));
    assert_eq!(filter.schema(), node(&plan, "Scan").schema());
}

#[test]
fn e4_4_step_4_a_date_range_on_a_timestamp_ends_before_the_day_after_to() {
    let bound = query(
        Vec::new(),
        vec![metric_reference("ORDER_COUNT", 0)],
        BoundQueryParts {
            date_range: Some(date_range(
                "ORDERED_AT",
                date(2026, 2, 1),
                date(2026, 2, 28),
            )),
            ..BoundQueryParts::default()
        },
    );
    let plan = build_plan(&branches(&[1]), &bound, &anyone(), &model(Vec::new())).expect("a plan");
    let filter = node(&plan, "Filter");
    assert_eq!(
        predicate(filter),
        &tree(r#""ORDERED_AT" >= DATE '2026-02-01' AND "ORDERED_AT" < DATE '2026-03-01'"#)
    );
    assert_eq!(filter.origin(), &Origin::Query(at("/query/date_range")));
}

#[test]
fn e4_4_step_4_a_date_range_ending_a_year_ends_before_january_first() {
    let bound = query(
        Vec::new(),
        vec![metric_reference("ORDER_COUNT", 0)],
        BoundQueryParts {
            date_range: Some(date_range(
                "ORDERED_AT",
                date(2025, 12, 1),
                date(2025, 12, 31),
            )),
            ..BoundQueryParts::default()
        },
    );
    let plan = build_plan(&branches(&[1]), &bound, &anyone(), &model(Vec::new())).expect("a plan");
    assert_eq!(
        predicate(node(&plan, "Filter")),
        &tree(r#""ORDERED_AT" >= DATE '2025-12-01' AND "ORDERED_AT" < DATE '2026-01-01'"#)
    );
}

#[test]
fn e4_4_step_4_a_query_without_filters_has_no_query_filter() {
    let bound = query(
        vec![dimension("REGION", None, 0)],
        vec![metric_reference("ORDER_COUNT", 0)],
        BoundQueryParts::default(),
    );
    let plan = build_plan(&branches(&[1]), &bound, &anyone(), &model(Vec::new())).expect("a plan");
    assert_eq!(kinds(&plan), ["Project", "Aggregate", "Scan"]);
}

#[test]
fn e4_4_step_6_the_aggregate_is_keyed_by_the_dimensions_with_a_time_grain() {
    let plan = sales_plan();
    let (keys, components) = aggregate_parts(&plan);
    assert_eq!(
        keys,
        [
            named("dimension_1", r#""REGION""#),
            named("dimension_2", r#"DATE_TRUNC('month', "ORDER_DATE")"#),
        ]
    );
    let calls: Vec<(&str, String)> = components
        .iter()
        .map(|component| {
            let call = menura_model::expression::Expression::Aggregate {
                call: component.call.clone(),
                filter: component.filter.clone().map(Box::new),
            };
            (component.name.as_str(), format!("{call:?}"))
        })
        .collect();
    let expected = |text: &str| format!("{:?}", tree(text));
    assert_eq!(
        calls,
        [
            ("metric_1_component_1", expected(r#"SUM("AMOUNT")"#)),
            ("metric_1_component_2", expected("COUNT(*)")),
            ("metric_2_component_1", expected("COUNT(*)")),
        ]
    );
}

#[test]
fn e4_4_step_6_a_component_keeps_its_filter_over_the_scan_columns() {
    let bound = query(
        Vec::new(),
        vec![metric_reference("PAID_AMOUNT", 0)],
        BoundQueryParts::default(),
    );
    let plan = build_plan(&branches(&[1]), &bound, &anyone(), &model(Vec::new())).expect("a plan");
    let (_, components) = aggregate_parts(&plan);
    assert_eq!(
        components
            .first()
            .and_then(|component| component.filter.clone()),
        Some(tree(r#""STATUS" = 'paid'"#))
    );
    assert_eq!(column_names(node(&plan, "Scan")), ["AMOUNT", "STATUS"]);
}

#[test]
fn e4_4_1_the_null_group_of_a_dimension_is_kept() {
    let plan = sales_plan();
    let aggregate = node(&plan, "Aggregate");
    assert_eq!(
        aggregate.schema().columns().first(),
        Some(&column("dimension_1", LogicalType::Text, true))
    );
    assert_eq!(
        predicate(node(&plan, "Filter")),
        &tree(
            r#""STATUS" = 'paid' AND ("ORDER_DATE" >= DATE '2026-01-01' AND "ORDER_DATE" <= DATE '2026-03-31')"#
        )
    );
}

#[test]
fn e4_4_a_dimension_only_plan_is_an_aggregate_with_keys_and_no_components() {
    let bound = query(
        vec![dimension("REGION", None, 0)],
        Vec::new(),
        BoundQueryParts::default(),
    );
    let plan = build_plan(&branches(&[]), &bound, &anyone(), &model(Vec::new())).expect("a plan");
    let (keys, components) = aggregate_parts(&plan);
    assert_eq!(keys, [named("dimension_1", r#""REGION""#)]);
    assert!(components.is_empty());
    assert_eq!(column_names(&plan), ["region"]);
}

#[test]
fn e4_4_a_metrics_only_plan_has_no_keys() {
    let bound = query(
        Vec::new(),
        vec![metric_reference("ORDER_COUNT", 0)],
        BoundQueryParts::default(),
    );
    let plan = build_plan(&branches(&[1]), &bound, &anyone(), &model(Vec::new())).expect("a plan");
    let (keys, components) = aggregate_parts(&plan);
    assert!(keys.is_empty());
    assert_eq!(components.len(), 1);
    assert_eq!(
        plan.schema().columns(),
        [column("order_count", big_integer(), true)]
    );
    assert_eq!(column_names(node(&plan, "Scan")), Vec::<&str>::new());
}

#[test]
fn e4_4_step_4_a_date_range_ending_9999_12_31_on_a_timestamp_has_no_upper_bound() {
    let bound = query(
        Vec::new(),
        vec![metric_reference("ORDER_COUNT", 0)],
        BoundQueryParts {
            date_range: Some(date_range(
                "ORDERED_AT",
                date(9999, 12, 1),
                date(9999, 12, 31),
            )),
            ..BoundQueryParts::default()
        },
    );
    let plan = build_plan(&branches(&[1]), &bound, &anyone(), &model(Vec::new())).expect("a plan");
    assert_eq!(
        predicate(node(&plan, "Filter")),
        &tree(r#""ORDERED_AT" >= DATE '9999-12-01'"#)
    );
}

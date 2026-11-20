//! The plan of the single branch of the 0.3.0 slice and its output (E4-1 to E4-4, E4-9).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_contracts::types::common::{Grain, LogicalType};
use menura_contracts::types::query::{Direction, NullsOrder};
use menura_e4_plan::build_plan;
use menura_expressions::typing::big_integer;
use menura_representations::bound_query::BoundQueryParts;
use menura_representations::logical_plan::{
    AggregateComponent, NamedExpression, Operation, PlanNode, SortItem,
};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;
use support::plan::{column, column_names, column_reference, kinds, node, predicate};
use support::{
    DATASET_POINTER, anyone, at, branches, date, date_range, dimension, filter, limit, located,
    metric_reference, model, order, query, tree,
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

fn project_expressions(plan: &PlanNode) -> &[NamedExpression] {
    match node(plan, "Project").operation() {
        Operation::Project { expressions, .. } => expressions,
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
        Operation::Scan { dataset } if dataset.as_str() == "ORDERS"
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
fn e4_4_the_project_evaluates_each_metric_from_its_components_in_output_order() {
    let plan = sales_plan();
    let project = node(&plan, "Project");
    assert_eq!(
        project_expressions(&plan),
        [
            NamedExpression {
                name: ColumnName::new("region"),
                expression: column_reference("dimension_1"),
            },
            NamedExpression {
                name: ColumnName::new("order_date"),
                expression: column_reference("dimension_2"),
            },
            named(
                "average_amount",
                r#""metric_1_component_1" / "metric_1_component_2""#
            ),
            NamedExpression {
                name: ColumnName::new("order_count"),
                expression: column_reference("metric_2_component_1"),
            },
        ]
    );
    assert_eq!(
        column_names(project),
        ["region", "order_date", "average_amount", "order_count"]
    );
    assert_eq!(
        project.schema().columns().get(1),
        Some(&column("order_date", LogicalType::Date, false))
    );
}

#[test]
fn e4_9_1_the_sort_uses_the_explicit_items_and_the_limit_follows_it() {
    let mut bound = sales_query();
    bound = query(
        bound.columns().dimensions().cloned().collect(),
        bound.columns().metrics().cloned().collect(),
        BoundQueryParts {
            order: vec![
                order(3, Direction::Desc, NullsOrder::Last, 0),
                order(0, Direction::Asc, NullsOrder::First, 1),
                order(1, Direction::Asc, NullsOrder::First, 2),
            ],
            limit: Some(limit(10)),
            ..BoundQueryParts::default()
        },
    );
    let plan =
        build_plan(&branches(&[2, 1]), &bound, &anyone(), &model(Vec::new())).expect("a plan");
    assert_eq!(
        kinds(&plan),
        ["Limit", "Sort", "Project", "Aggregate", "Scan"]
    );
    assert!(matches!(
        plan.operation(),
        Operation::Limit { rows: 10, .. }
    ));
    assert_eq!(plan.origin(), &Origin::Query(at("/query/limit")));
    let sort = node(&plan, "Sort");
    let Operation::Sort { items, .. } = sort.operation() else {
        unreachable!()
    };
    let item = |name: &str, direction, nulls| SortItem {
        column: ColumnName::new(name),
        direction,
        nulls,
    };
    assert_eq!(
        items,
        &[
            item("order_count", Direction::Desc, NullsOrder::Last),
            item("region", Direction::Asc, NullsOrder::First),
            item("order_date", Direction::Asc, NullsOrder::First),
        ]
    );
    assert_eq!(sort.origin(), &Origin::Query(at("/query/order_by/0")));
}

#[test]
fn e4_9_2_without_order_by_the_plan_has_no_sort() {
    let bound = query(
        vec![dimension("REGION", None, 0)],
        vec![metric_reference("ORDER_COUNT", 0)],
        BoundQueryParts::default(),
    );
    let plan = build_plan(&branches(&[1]), &bound, &anyone(), &model(Vec::new())).expect("a plan");
    assert_eq!(kinds(&plan).first(), Some(&"Project"));
}

#[test]
fn e4_3_every_node_carries_its_output_schema_and_origin() {
    let plan = sales_plan();
    let schemas: Vec<Vec<&str>> = support::plan::chain(&plan)
        .into_iter()
        .map(column_names)
        .collect();
    assert_eq!(
        schemas,
        [
            vec!["region", "order_date", "average_amount", "order_count"],
            vec!["region", "order_date", "average_amount", "order_count"],
            vec![
                "dimension_1",
                "dimension_2",
                "metric_1_component_1",
                "metric_1_component_2",
                "metric_2_component_1",
            ],
            vec!["AMOUNT", "ORDER_DATE", "REGION", "STATUS"],
            vec!["AMOUNT", "ORDER_DATE", "REGION", "STATUS"],
        ]
    );
    let aggregate = node(&plan, "Aggregate");
    assert_eq!(
        aggregate.schema().columns().get(4),
        Some(&column("metric_2_component_1", big_integer(), false))
    );
    assert_eq!(
        node(&plan, "Sort").origin(),
        &Origin::Query(at("/query/order_by/0"))
    );
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

#[test]
fn e4_3_the_aggregate_and_project_come_from_the_model() {
    let plan = sales_plan();
    assert_eq!(
        node(&plan, "Aggregate").origin(),
        &Origin::Model(located(DATASET_POINTER))
    );
    assert_eq!(
        node(&plan, "Project").origin(),
        &Origin::Model(located("/semantic_model/0/metrics/AVERAGE_AMOUNT"))
    );
    let dimension_only = query(
        vec![dimension("REGION", None, 0)],
        Vec::new(),
        BoundQueryParts::default(),
    );
    let plan = build_plan(
        &branches(&[]),
        &dimension_only,
        &anyone(),
        &model(Vec::new()),
    )
    .expect("a plan");
    assert_eq!(plan.origin(), &Origin::Model(located(DATASET_POINTER)));
}

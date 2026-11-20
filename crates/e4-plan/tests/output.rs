//! The output of the plan: `Project`, `Sort` and `Limit`, and every node's schema and origin
//! (E4-3, E4-9).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_contracts::types::common::{Grain, LogicalType};
use menura_contracts::types::query::{Direction, NullsOrder};
use menura_e4_plan::build_plan;
use menura_expressions::typing::big_integer;
use menura_representations::bound_query::BoundQueryParts;
use menura_representations::logical_plan::{NamedExpression, Operation, PlanNode, SortItem};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;
use support::plan::{column, column_names, column_reference, kinds, node};
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

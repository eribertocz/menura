//! The plan of a query of the SQL form: expression dimensions, `SELECT` order, `ORDER BY` and
//! `LIMIT` (E1-4.1, E1-4.3, E4-9.1).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_contracts::types::common::LogicalType;
use menura_contracts::types::query::{Direction, NullsOrder};
use menura_e4_plan::{InvariantViolation, PlanError};
use menura_representations::bound_query::{BoundOrderItem, BoundQueryParts};
use menura_representations::logical_plan::{Operation, PlanNode};
use menura_representations::query_tree::QueryLimit;
use support::plan::{column_names, kinds, node};
use support::sql::{SqlColumn, aggregated, expression_dimension, select_item, sql_query};
use support::{anyone, branches, build_plan, model, tree};

fn plan(columns: Vec<SqlColumn>, parts: BoundQueryParts) -> Result<PlanNode, PlanError> {
    let metric_count = columns
        .iter()
        .filter(|column| matches!(column, SqlColumn::Metric(_)))
        .count();
    let query = sql_query(columns, parts);
    build_plan(
        &branches(&vec![1; metric_count]),
        &query,
        &anyone(),
        &model(Vec::new()),
    )
}

fn by_channel() -> Vec<SqlColumn> {
    vec![
        expression_dimension(r#"UPPER("ORDERS"."REGION")"#, "r", 8),
        aggregated("TOTAL_AMOUNT", "amount", 35),
    ]
}

#[test]
fn e1_4_1_an_expression_dimension_is_an_aggregate_key_over_the_scan() {
    let plan = plan(by_channel(), BoundQueryParts::default()).expect("a plan");

    assert_eq!(kinds(&plan), ["Project", "Aggregate", "Scan"]);
    let Operation::Aggregate { keys, .. } = node(&plan, "Aggregate").operation() else {
        panic!("an Aggregate");
    };
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].expression, tree(r#"UPPER("REGION")"#));
    let r = &plan.schema().columns()[0];
    assert_eq!(r.logical_type, LogicalType::Text);
    assert!(r.nullable);
}

#[test]
fn e1_4_3_result_columns_keep_the_select_order() {
    let columns = vec![
        aggregated("ORDER_COUNT", "orders", 8),
        expression_dimension(r#"LOWER("ORDERS"."STATUS")"#, "status", 30),
        aggregated("TOTAL_AMOUNT", "amount", 60),
    ];

    let plan = plan(columns, BoundQueryParts::default()).expect("a plan");

    assert_eq!(column_names(&plan), ["orders", "status", "amount"]);
}

#[test]
fn e1_4_1_a_query_without_dimensions_is_one_aggregate_without_keys() {
    let plan = plan(
        vec![aggregated("TOTAL_AMOUNT", "amount", 8)],
        BoundQueryParts::default(),
    )
    .expect("a plan");

    let Operation::Aggregate { keys, .. } = node(&plan, "Aggregate").operation() else {
        panic!("an Aggregate");
    };
    assert!(keys.is_empty());
    assert_eq!(column_names(&plan), ["amount"]);
}

#[test]
fn e4_9_1_order_by_and_limit_sort_the_output_columns_they_name() {
    let parts = BoundQueryParts {
        order: vec![BoundOrderItem {
            column: 0,
            direction: Direction::Desc,
            nulls: NullsOrder::Last,
            origin: select_item(70),
        }],
        limit: Some(QueryLimit {
            rows: 3,
            origin: select_item(90),
        }),
        ..BoundQueryParts::default()
    };

    let plan = plan(by_channel(), parts).expect("a plan");

    assert_eq!(
        kinds(&plan),
        ["Limit", "Sort", "Project", "Aggregate", "Scan"]
    );
    let Operation::Sort { items, .. } = node(&plan, "Sort").operation() else {
        panic!("a Sort");
    };
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].column.as_str(), "r");
    assert_eq!(items[0].direction, Direction::Desc);
}

#[test]
fn e1_4_1_an_expression_dimension_over_another_dataset_is_an_internal_error() {
    let columns = vec![
        expression_dimension(r#"UPPER("CUSTOMERS"."REGION")"#, "r", 8),
        aggregated("TOTAL_AMOUNT", "amount", 35),
    ];

    let result = plan(columns, BoundQueryParts::default());

    let Err(PlanError::Invariant { violation, .. }) = result else {
        panic!("an internal error, got {result:?}");
    };
    assert_eq!(violation, InvariantViolation::ForeignField);
}

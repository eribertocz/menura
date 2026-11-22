//! E1-4.9, E2-3.3 and E2-5.3: the errors of a SQL-form query that need the model, at rows 3 to
//! 9 of E2-5, each at its SQL location, in SQL query order.

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_e2_bind::BindCode;
use menura_model::expression::{BinaryOperator, Expression};

use support::diagnostic;
use support::query::{column, equals_text, metric_aggregate};
use support::sql::{
    agg_item, expression_item, field_item, inline_item, relation, sql_at, sql_columns, sql_where,
    upper, where_filter,
};

const ANALYST: &[&str] = &["analyst"];

/// The code and the SQL column of the first error of `tree` for an analyst.
#[cfg(test)]
fn first_error_at(tree: &menura_representations::query_tree::QueryTree) -> (BindCode, usize) {
    let found = diagnostic(tree, ANALYST);
    assert_eq!(found.location.pointer(), "/sql");
    let column = found
        .location
        .position()
        .map(menura_representations::origin::TextPosition::column)
        .expect("a position in the SQL text");
    (found.code, column)
}

#[test]
fn e1_4_9_a_from_model_that_is_not_the_model_name_is_name_not_found() {
    let tree = sql_columns(
        relation("other", "orders", None, 30),
        vec![agg_item("revenue", "r", 8)],
    );
    assert_eq!(first_error_at(&tree), (BindCode::NameNotFound, 30));
}

#[test]
fn qry_5_1_an_unknown_dataset_is_name_not_found() {
    let tree = sql_columns(
        relation("sales", "nowhere", None, 30),
        vec![agg_item("revenue", "r", 8)],
    );
    assert_eq!(first_error_at(&tree), (BindCode::NameNotFound, 30));
}

#[test]
fn qry_5_1_an_unauthorized_dataset_is_reported_as_an_unknown_one() {
    let unauthorized = sql_columns(
        relation("sales", "secrets", None, 30),
        vec![agg_item("secret_total", "s", 8)],
    );
    let unknown = sql_columns(
        relation("sales", "secrets_", None, 30),
        vec![agg_item("secret_total", "s", 8)],
    );
    let denied = diagnostic(&unauthorized, ANALYST);
    let missing = diagnostic(&unknown, ANALYST);
    assert_eq!(denied.code, BindCode::NameNotFound);
    assert_eq!(
        denied.message.replace("secrets", "secrets_"),
        missing.message
    );
}

#[test]
fn qry_5_1_a_summary_dataset_cannot_be_named() {
    let tree = sql_columns(
        relation("sales", "orders_summary", None, 30),
        vec![field_item("region", 8), agg_item("revenue", "r", 16)],
    );
    assert_eq!(first_error_at(&tree), (BindCode::NameNotFound, 30));
}

#[test]
fn qry_5_1_a_qualifier_other_than_the_alias_or_the_dataset_is_name_not_found() {
    let tree = sql_columns(
        relation("sales", "orders", Some("o"), 30),
        vec![
            field_item("customers.region", 8),
            agg_item("revenue", "r", 26),
        ],
    );
    assert_eq!(first_error_at(&tree), (BindCode::NameNotFound, 8));
}

#[test]
fn qry_5_2_a_metric_not_exposed_on_the_relation_is_name_not_found() {
    let tree = sql_columns(
        relation("sales", "orders", None, 30),
        vec![agg_item("secret_total", "s", 8)],
    );
    assert_eq!(first_error_at(&tree), (BindCode::NameNotFound, 8));
}

#[test]
fn e1_4_9_a_metric_named_without_agg_is_metric_in_dimensions() {
    let tree = sql_columns(
        relation("sales", "orders", None, 40),
        vec![
            field_item("region", 8),
            field_item("revenue", 16),
            agg_item("order_total", "c", 25),
        ],
    );
    assert_eq!(first_error_at(&tree), (BindCode::MetricInDimensions, 16));
}

#[test]
fn e1_4_9_agg_of_a_field_is_field_in_metrics() {
    let tree = sql_columns(
        relation("sales", "orders", None, 40),
        vec![field_item("region", 8), agg_item("amount", "a", 16)],
    );
    assert_eq!(first_error_at(&tree), (BindCode::FieldInMetrics, 16));
}

#[test]
fn e2_3_3_an_expression_dimension_that_reads_a_metric_is_metric_in_dimensions() {
    let tree = sql_columns(
        relation("sales", "orders", None, 50),
        vec![
            expression_item(upper(column("revenue")), "x", 8),
            agg_item("order_total", "c", 30),
        ],
    );
    assert_eq!(first_error_at(&tree), (BindCode::MetricInDimensions, 8));
}

#[test]
fn e2_3_3_an_inline_metric_over_agg_of_a_field_is_field_in_metrics() {
    let sum = Expression::Binary {
        operator: BinaryOperator::Add,
        left: Box::new(metric_aggregate("revenue")),
        right: Box::new(metric_aggregate("amount")),
    };
    let tree = sql_columns(
        relation("sales", "orders", None, 50),
        vec![inline_item(sum, "s", 8)],
    );
    assert_eq!(first_error_at(&tree), (BindCode::FieldInMetrics, 8));
}

#[test]
fn e1_4_9_a_where_over_a_metric_is_aggregate_in_where() {
    let tree = sql_where(
        relation("sales", "orders", None, 30),
        vec![agg_item("revenue", "r", 8)],
        where_filter(equals_text(column("revenue")), 50),
    );
    assert_eq!(first_error_at(&tree), (BindCode::AggregateInWhere, 50));
}

#[test]
fn e1_4_9_an_unauthorized_field_is_reported_as_an_unknown_one() {
    let tree = sql_columns(
        relation("sales", "orders", None, 40),
        vec![field_item("unit_cost", 8), agg_item("revenue", "r", 20)],
    );
    assert_eq!(first_error_at(&tree), (BindCode::NameNotFound, 8));
}

#[test]
fn e2_5_3_the_relation_is_checked_before_the_select_items() {
    let tree = sql_columns(
        relation("other", "orders", None, 40),
        vec![field_item("nowhere", 8), agg_item("revenue", "r", 20)],
    );
    assert_eq!(first_error_at(&tree), (BindCode::NameNotFound, 40));
}

#[test]
fn e2_5_3_the_select_items_are_checked_before_where() {
    let tree = sql_where(
        relation("sales", "orders", None, 40),
        vec![agg_item("nowhere", "n", 8)],
        where_filter(equals_text(column("elsewhere")), 60),
    );
    assert_eq!(first_error_at(&tree), (BindCode::NameNotFound, 8));
}

#[test]
fn e2_5_3_each_row_is_checked_over_every_item_before_the_next_row() {
    let tree = sql_columns(
        relation("sales", "orders", None, 50),
        vec![
            agg_item("amount", "a", 8),
            field_item("revenue", 20),
            agg_item("order_total", "c", 30),
        ],
    );
    assert_eq!(first_error_at(&tree), (BindCode::MetricInDimensions, 20));
}

#[test]
fn e2_5_3_references_of_an_expression_are_taken_in_text_order() {
    let condition = Expression::Binary {
        operator: BinaryOperator::Or,
        left: Box::new(equals_text(column("first_unknown"))),
        right: Box::new(equals_text(column("region"))),
    };
    let tree = sql_columns(
        relation("sales", "orders", None, 60),
        vec![
            expression_item(condition, "flag", 8),
            agg_item("revenue", "r", 50),
        ],
    );
    let found = diagnostic(&tree, ANALYST);
    assert_eq!(found.code, BindCode::NameNotFound);
    assert_eq!(found.location, sql_at(8));
    assert!(found.message.contains("first_unknown"));
}

#[test]
fn e2_3_3_an_expression_dimension_that_reads_no_field_is_metric_in_dimensions() {
    let one = Expression::Literal(menura_model::expression::Literal::Boolean(true));
    let tree = sql_columns(
        relation("sales", "orders", None, 40),
        vec![
            expression_item(one, "flag", 8),
            agg_item("revenue", "r", 20),
        ],
    );
    assert_eq!(first_error_at(&tree), (BindCode::MetricInDimensions, 8));
}

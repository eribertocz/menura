//! E2-5 rows 3 to 9, E2-5.1, E2-5.2, E2-5.3 and E2-5.5: the first error in the order of the
//! table, and within a row the first element in query order.

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_e2_bind::BindCode;
use menura_model::expression::{Expression, Identifier, Reference};
use menura_representations::names::OutputName;
use menura_representations::query_tree::{QueryTreeParts, ReferencePart};

use support::query::{
    column, count_rows_filter, date_range, dimension, dimension_reference, equals_text, filter,
    metric, parts, path, tree,
};
use support::{first_error, run};

const ANALYST: &[&str] = &["analyst"];

fn error(code: BindCode, pointer: &str) -> (BindCode, String) {
    (code, pointer.to_owned())
}

/// `orders.region = customers.region`: a filter over two datasets.
fn cross_dataset() -> Expression {
    Expression::Binary {
        operator: menura_model::expression::BinaryOperator::Equal,
        left: Box::new(column("orders.region")),
        right: Box::new(column("customers.region")),
    }
}

fn filters(items: Vec<Expression>) -> QueryTreeParts {
    QueryTreeParts {
        filters: items
            .into_iter()
            .enumerate()
            .map(|(index, expression)| {
                filter(
                    &format!("/query/nonportable_extensions/0/data/filters/{index}"),
                    expression,
                )
            })
            .collect(),
        ..QueryTreeParts::default()
    }
}

#[test]
fn e2_5_row_3_an_unknown_dimension_and_metric_report_the_dimension() {
    let query = tree(
        vec![dimension(0, "orders.colour")],
        vec![metric(0, "profit")],
        QueryTreeParts::default(),
    );
    assert_eq!(
        first_error(&query, ANALYST),
        error(BindCode::NameNotFound, "/query/dimensions/0")
    );
}

#[test]
fn e2_5_rows_3_and_4_name_not_found_comes_first_even_later_in_query_order() {
    let query = tree(
        vec![dimension(0, "revenue")],
        vec![],
        filters(vec![equals_text(column("orders.colour"))]),
    );
    assert_eq!(
        first_error(&query, ANALYST),
        error(
            BindCode::NameNotFound,
            "/query/nonportable_extensions/0/data/filters/0"
        )
    );
}

#[test]
fn e2_5_rows_4_and_5_metric_in_dimensions_comes_first() {
    let query = tree(
        vec![dimension(0, "revenue")],
        vec![metric(0, "orders.amount")],
        QueryTreeParts::default(),
    );
    assert_eq!(
        first_error(&query, ANALYST),
        error(BindCode::MetricInDimensions, "/query/dimensions/0")
    );
}

#[test]
fn e2_5_rows_5_and_6_field_in_metrics_comes_first() {
    let query = tree(
        vec![],
        vec![metric(0, "orders.amount")],
        filters(vec![count_rows_filter()]),
    );
    assert_eq!(
        first_error(&query, ANALYST),
        error(BindCode::FieldInMetrics, "/query/metrics/0")
    );
}

#[test]
fn e2_5_row_6_an_aggregate_or_a_metric_in_a_filter_is_aggregate_in_where() {
    let metric_reference = Expression::MetricAggregate(Reference {
        qualifier: None,
        name: Identifier::regular("revenue").expect("an identifier"),
    });
    for expression in [
        count_rows_filter(),
        equals_text(metric_reference),
        equals_text(column("orders.order_total")),
    ] {
        let query = tree(
            vec![dimension(0, "orders.region")],
            vec![],
            filters(vec![expression]),
        );
        assert_eq!(
            first_error(&query, ANALYST),
            error(
                BindCode::AggregateInWhere,
                "/query/nonportable_extensions/0/data/filters/0"
            )
        );
    }
}

#[test]
fn e2_5_rows_6_and_8_aggregate_in_where_comes_first_even_later_in_query_order() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        filters(vec![cross_dataset(), count_rows_filter()]),
    );
    assert_eq!(
        first_error(&query, ANALYST),
        error(
            BindCode::AggregateInWhere,
            "/query/nonportable_extensions/0/data/filters/1"
        )
    );
}

#[test]
fn e2_5_row_8_a_filter_over_two_datasets_is_cross_dataset_filter() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        filters(vec![equals_text(column("orders.region")), cross_dataset()]),
    );
    assert_eq!(
        first_error(&query, ANALYST),
        error(
            BindCode::CrossDatasetFilter,
            "/query/nonportable_extensions/0/data/filters/1"
        )
    );
}

#[test]
fn e2_5_rows_8_and_9_cross_dataset_filter_comes_first_even_later_in_query_order() {
    let query = tree(
        vec![
            dimension(0, "orders.region"),
            dimension(1, "customers.region"),
        ],
        vec![],
        filters(vec![cross_dataset()]),
    );
    assert_eq!(
        first_error(&query, ANALYST),
        error(
            BindCode::CrossDatasetFilter,
            "/query/nonportable_extensions/0/data/filters/0"
        )
    );
}

#[test]
fn e2_5_row_9_a_repeated_output_name_is_reported_at_the_later_column() {
    let query = tree(
        vec![
            dimension(0, "orders.region"),
            dimension(1, "orders.amount"),
            dimension(2, "customers.region"),
        ],
        vec![],
        QueryTreeParts::default(),
    );
    assert_eq!(
        first_error(&query, ANALYST),
        error(BindCode::NameCollision, "/query/dimensions/2")
    );
}

#[test]
fn e2_5_5_output_names_compare_after_the_upper_case_mapping() {
    let mut aliased = dimension(1, "customers.region");
    aliased.output_name = OutputName::new("Region");
    let query = tree(
        vec![dimension(0, "orders.region"), aliased],
        vec![],
        QueryTreeParts::default(),
    );
    assert_eq!(
        first_error(&query, ANALYST),
        error(BindCode::NameCollision, "/query/dimensions/1")
    );
    let segment = parts(vec![
        ReferencePart::Regular("customers".to_owned()),
        ReferencePart::Delimited("Segment".to_owned()),
    ]);
    let mut total = metric(0, "revenue");
    total.output_name = OutputName::new("SEGMENT");
    let query = tree(
        vec![dimension_reference(0, segment)],
        vec![total],
        QueryTreeParts::default(),
    );
    assert_eq!(
        first_error(&query, ANALYST),
        error(BindCode::NameCollision, "/query/metrics/0")
    );
}

#[test]
fn e2_5_5_distinct_output_names_do_not_collide() {
    let mut aliased = dimension(1, "customers.region");
    aliased.output_name = OutputName::new("customer_region");
    let query = tree(
        vec![dimension(0, "orders.region"), aliased],
        vec![metric(0, "revenue")],
        QueryTreeParts::default(),
    );
    assert!(run(&query, ANALYST).is_ok());
}

#[test]
fn e2_5_1_names_are_checked_in_e2_whatever_e1_accepted() {
    let query = tree(
        vec![dimension(0, "revenue"), dimension(1, "a.b.c")],
        vec![],
        QueryTreeParts::default(),
    );
    assert_eq!(
        first_error(&query, ANALYST),
        error(BindCode::NameNotFound, "/query/dimensions/1")
    );
}

#[test]
fn e2_5_2_only_the_first_error_is_returned() {
    let query = tree(
        vec![dimension(0, "orders.colour"), dimension(1, "orders.size")],
        vec![metric(0, "orders.amount")],
        QueryTreeParts::default(),
    );
    assert_eq!(
        first_error(&query, ANALYST),
        error(BindCode::NameNotFound, "/query/dimensions/0")
    );
}

#[test]
fn e2_5_3_the_date_range_comes_before_paths_whatever_their_place_in_the_document() {
    let query = |range_index: usize, paths_index: usize| {
        let parts = QueryTreeParts {
            date_range: Some(date_range(
                &format!("/query/nonportable_extensions/{range_index}/data/date_range"),
                "orders.shipped_at",
            )),
            paths: vec![path(
                &format!("/query/nonportable_extensions/{paths_index}/data/paths/0"),
                "orders",
                "clients",
                &["orders_to_customers"],
            )],
            ..QueryTreeParts::default()
        };
        tree(vec![dimension(0, "orders.region")], vec![], parts)
    };
    assert_eq!(
        first_error(&query(0, 1), ANALYST),
        error(
            BindCode::NameNotFound,
            "/query/nonportable_extensions/0/data/date_range"
        )
    );
    assert_eq!(
        first_error(&query(1, 0), ANALYST),
        error(
            BindCode::NameNotFound,
            "/query/nonportable_extensions/1/data/date_range"
        )
    );
}

#[test]
fn e2_5_3_shuffled_json_keys_give_the_same_first_error() {
    let written_first = tree(
        vec![dimension(0, "orders.colour")],
        vec![metric(0, "profit")],
        filters(vec![equals_text(column("orders.size"))]),
    );
    let mut shuffled_parts = filters(vec![equals_text(column("orders.size"))]);
    shuffled_parts.filters[0] = filter("/query/filter", equals_text(column("orders.size")));
    let shuffled = tree(
        vec![dimension(0, "orders.colour")],
        vec![metric(0, "profit")],
        shuffled_parts,
    );
    assert_eq!(
        first_error(&written_first, ANALYST),
        first_error(&shuffled, ANALYST)
    );
    assert_eq!(
        first_error(&shuffled, ANALYST),
        error(BindCode::NameNotFound, "/query/dimensions/0")
    );
}

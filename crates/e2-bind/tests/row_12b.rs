//! E2-5 row 12b, in its order (`E2-01`, `EQRY-09`, `EQRY-10`, `EQRY-11`, `EQRY-12`, `EQRY-03`,
//! `EQRY-04`): E2-3.4, QRY-4.1, QRY-4.2, QRY-4.5, QRY-4.6, and the bound query they build.

#[cfg(test)]
mod support;

use menura_contracts::types::common::Grain;
use menura_e2_bind::BindCode;
use menura_representations::bound_query::BoundDimensionKind;
use menura_representations::names::OutputName;
use menura_representations::query_tree::{QueryDimension, QueryTreeParts};
use menura_representations::time::{QueryTime, TimeZoneName};

use support::model::model;
use support::query::{
    absolute_range, date, dimension, limit, metric, order_item, path, relative_range, time_grain,
    tree, tree_at,
};
use support::{bind_error, bind_query};

const ANALYST: &[&str] = &["analyst"];
const ADMIN: &[&str] = &["admin"];
const GRAINS: &str = "/query/nonportable_extensions/0/data/time_grains";
const RANGE: &str = "/query/nonportable_extensions/0/data/date_range";
const PATHS: &str = "/query/nonportable_extensions/0/data/paths";
const LIMIT: &str = "/query/nonportable_extensions/0/data/limit";

fn error(code: BindCode, pointer: &str) -> (BindCode, String) {
    (code, pointer.to_owned())
}

fn grain(
    index: usize,
    text: &str,
    grain: Grain,
) -> menura_representations::query_tree::TimeGrainChoice {
    time_grain(&format!("{GRAINS}/{index}"), text, grain)
}

/// The dimension `text` with the alias `alias`.
fn aliased(index: usize, text: &str, alias: &str) -> QueryDimension {
    let mut dimension = dimension(index, text);
    dimension.output_name = OutputName::new(alias);
    dimension
}

#[test]
fn e2_3_4_a_time_grain_on_a_field_that_is_not_a_time_field_is_e2_01() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        QueryTreeParts {
            time_grains: vec![grain(0, "orders.region", Grain::Month)],
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model(), ANALYST),
        error(BindCode::NotTimeField, &format!("{GRAINS}/0"))
    );
}

#[test]
fn e2_3_4_a_date_range_on_a_metric_is_e2_01() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        QueryTreeParts {
            date_range: Some(absolute_range(RANGE, "revenue", "2026-01-01", "2026-01-31")),
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model(), ANALYST),
        error(BindCode::NotTimeField, RANGE)
    );
}

#[test]
fn e2_3_4_a_time_field_accepts_a_grain_and_a_range() {
    let query = tree(
        vec![dimension(0, "orders.ordered_at")],
        vec![],
        QueryTreeParts {
            time_grains: vec![grain(0, "orders.ordered_at", Grain::Month)],
            date_range: Some(absolute_range(
                RANGE,
                "orders.ordered_at",
                "2026-01-01",
                "2026-03-31",
            )),
            ..QueryTreeParts::default()
        },
    );
    let bound = bind_query(&query, &model(), ANALYST).expect("a bound query");
    let dimension = bound.columns().dimensions().next().expect("a dimension");
    assert!(matches!(
        dimension.kind,
        BoundDimensionKind::Field {
            time_grain: Some(Grain::Month),
            ..
        }
    ));
    let range = bound.date_range().expect("a date range");
    assert_eq!(
        (range.from, range.to),
        (date(2026, 1, 1), date(2026, 3, 31))
    );
}

#[test]
fn e2_5_rows_12b_e2_01_comes_before_eqry_09_even_later_in_query_order() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        QueryTreeParts {
            time_grains: vec![
                grain(0, "customers.region", Grain::Month),
                grain(1, "orders.region", Grain::Month),
            ],
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model(), ANALYST),
        error(BindCode::NotTimeField, &format!("{GRAINS}/0"))
    );
}

#[test]
fn qry_4_1_a_grain_on_a_field_that_is_not_a_dimension_is_eqry_09() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        QueryTreeParts {
            time_grains: vec![grain(0, "orders.ordered_at", Grain::Day)],
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model(), ANALYST),
        error(BindCode::TimeGrainNotDimension, &format!("{GRAINS}/0"))
    );
}

#[test]
fn qry_4_1_two_grains_on_one_dimension_is_eqry_09_at_the_second() {
    let query = tree(
        vec![dimension(0, "orders.ordered_at")],
        vec![],
        QueryTreeParts {
            time_grains: vec![
                grain(0, "orders.ordered_at", Grain::Day),
                grain(1, "orders.ordered_at", Grain::Year),
            ],
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model(), ANALYST),
        error(BindCode::TimeGrainNotDimension, &format!("{GRAINS}/1"))
    );
}

#[test]
fn e2_5_rows_12b_eqry_09_comes_before_eqry_10() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        QueryTreeParts {
            time_grains: vec![grain(0, "orders.ordered_at", Grain::Day)],
            order: vec![order_item(0, "colour")],
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model(), ANALYST),
        error(BindCode::TimeGrainNotDimension, &format!("{GRAINS}/0"))
    );
}

#[test]
fn qry_4_5_an_order_by_output_that_names_no_output_is_eqry_10() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        QueryTreeParts {
            order: vec![order_item(0, "REGION"), order_item(1, "colour")],
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model(), ANALYST),
        error(
            BindCode::UnknownOrderOutput,
            "/query/nonportable_extensions/0/data/order_by/1"
        )
    );
}

#[test]
fn qry_4_5_an_order_by_output_matches_an_output_name_ignoring_case() {
    let query = tree(
        vec![aliased(0, "orders.region", "Area")],
        vec![metric(0, "revenue")],
        QueryTreeParts {
            order: vec![order_item(0, "revenue"), order_item(1, "AREA")],
            ..QueryTreeParts::default()
        },
    );
    let bound = bind_query(&query, &model(), ANALYST).expect("a bound query");
    let columns: Vec<usize> = bound.order().iter().map(|item| item.column).collect();
    assert_eq!(columns, vec![1, 0]);
}

#[test]
fn e2_5_rows_12b_eqry_10_comes_before_eqry_11() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        QueryTreeParts {
            date_range: Some(absolute_range(
                RANGE,
                "orders.ordered_at",
                "2026-02-01",
                "2026-01-01",
            )),
            order: vec![order_item(0, "colour")],
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model(), ANALYST),
        error(
            BindCode::UnknownOrderOutput,
            "/query/nonportable_extensions/0/data/order_by/0"
        )
    );
}

#[test]
fn qry_4_2_an_absolute_range_whose_from_is_after_its_to_is_eqry_11() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        QueryTreeParts {
            date_range: Some(absolute_range(
                RANGE,
                "orders.ordered_at",
                "2026-02-01",
                "2026-01-31",
            )),
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model(), ANALYST),
        error(BindCode::ReversedDateRange, RANGE)
    );
}

#[test]
fn qry_4_2_an_absolute_range_of_one_day_is_accepted() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        QueryTreeParts {
            date_range: Some(absolute_range(
                RANGE,
                "orders.ordered_at",
                "2026-02-01",
                "2026-02-01",
            )),
            ..QueryTreeParts::default()
        },
    );
    let range = bind_query(&query, &model(), ANALYST)
        .expect("a bound query")
        .date_range()
        .cloned()
        .expect("a range");
    assert_eq!((range.from, range.to), (date(2026, 2, 1), date(2026, 2, 1)));
}

#[test]
fn e2_5_rows_12b_eqry_11_comes_before_eqry_12() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        QueryTreeParts {
            date_range: Some(absolute_range(
                RANGE,
                "orders.ordered_at",
                "2026-02-01",
                "2026-01-01",
            )),
            paths: vec![path(
                &format!("{PATHS}/0"),
                "orders",
                "secrets",
                &["orders_to_customers"],
            )],
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model(), ADMIN),
        error(BindCode::ReversedDateRange, RANGE)
    );
}

#[test]
fn qry_4_3_relationships_that_do_not_chain_from_from_to_to_are_eqry_12() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        QueryTreeParts {
            paths: vec![path(
                &format!("{PATHS}/0"),
                "orders",
                "secrets",
                &["orders_to_customers"],
            )],
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model(), ADMIN),
        error(BindCode::InvalidPath, &format!("{PATHS}/0"))
    );
}

#[test]
fn qry_4_3_two_paths_for_one_pair_are_eqry_12_at_the_second() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        QueryTreeParts {
            paths: vec![
                path(
                    &format!("{PATHS}/0"),
                    "orders",
                    "customers",
                    &["orders_to_customers"],
                ),
                path(
                    &format!("{PATHS}/1"),
                    "orders",
                    "customers",
                    &["orders_by_cost"],
                ),
            ],
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model(), ADMIN),
        error(BindCode::InvalidPath, &format!("{PATHS}/1"))
    );
}

#[test]
fn e2_5_rows_12b_eqry_12_comes_before_eqry_03() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        QueryTreeParts {
            date_range: Some(relative_range(
                RANGE,
                "orders.ordered_at",
                Grain::Day,
                7,
                None,
            )),
            paths: vec![path(
                &format!("{PATHS}/0"),
                "orders",
                "secrets",
                &["orders_to_customers"],
            )],
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model(), ADMIN),
        error(BindCode::InvalidPath, &format!("{PATHS}/0"))
    );
}

#[test]
fn qry_4_2_a_relative_range_without_reference_time_is_eqry_03() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        QueryTreeParts {
            date_range: Some(relative_range(
                RANGE,
                "orders.ordered_at",
                Grain::Day,
                7,
                None,
            )),
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model(), ANALYST),
        error(BindCode::MissingReferenceTime, RANGE)
    );
}

#[test]
fn e2_5_rows_12b_eqry_03_comes_before_eqry_04() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        QueryTreeParts {
            date_range: Some(relative_range(
                RANGE,
                "orders.ordered_at",
                Grain::Day,
                7,
                None,
            )),
            limit: Some(limit(10)),
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model(), ANALYST),
        error(BindCode::MissingReferenceTime, RANGE)
    );
}

#[test]
fn qry_4_6_a_limit_with_an_order_by_that_lacks_one_dimension_is_eqry_04() {
    let query = tree(
        vec![
            dimension(0, "orders.region"),
            dimension(1, "orders.ordered_at"),
        ],
        vec![metric(0, "revenue")],
        QueryTreeParts {
            order: vec![order_item(0, "region"), order_item(1, "revenue")],
            limit: Some(limit(10)),
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model(), ANALYST),
        error(BindCode::LimitWithoutTotalOrder, LIMIT)
    );
}

#[test]
fn qry_4_6_a_limit_with_every_dimension_ordered_is_accepted() {
    let query = tree(
        vec![
            dimension(0, "orders.region"),
            dimension(1, "orders.ordered_at"),
        ],
        vec![metric(0, "revenue")],
        QueryTreeParts {
            order: vec![order_item(0, "ordered_at"), order_item(1, "region")],
            limit: Some(limit(10)),
            ..QueryTreeParts::default()
        },
    );
    let bound = bind_query(&query, &model(), ANALYST).expect("a bound query");
    assert_eq!(bound.limit().map(|limit| limit.rows), Some(10));
}

#[test]
fn qry_4_6_a_limit_without_order_by_is_eqry_04() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        QueryTreeParts {
            limit: Some(limit(10)),
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model(), ANALYST),
        error(BindCode::LimitWithoutTotalOrder, LIMIT)
    );
}

#[test]
fn e2_5_rows_12b_and_12c_eqry_04_comes_before_not_yet_supported() {
    let query = tree(
        vec![dimension(0, "customers.region")],
        vec![metric(0, "revenue")],
        QueryTreeParts {
            limit: Some(limit(10)),
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model(), ANALYST),
        error(BindCode::LimitWithoutTotalOrder, LIMIT)
    );
}

#[test]
fn e2_2_the_bound_query_keeps_the_time_of_the_tree() {
    let time = QueryTime {
        time_zone: TimeZoneName::new("Europe/Madrid"),
        reference_time: None,
    };
    let query = tree_at(
        vec![dimension(0, "orders.region")],
        vec![],
        time.clone(),
        QueryTreeParts::default(),
    );
    let bound = bind_query(&query, &model(), ANALYST).expect("a bound query");
    assert_eq!(bound.time(), &time);
}

/// `orders.ordered_at >= CURRENT_DATE`.
fn since_today() -> menura_model::expression::Expression {
    menura_model::expression::Expression::Binary {
        operator: menura_model::expression::BinaryOperator::GreaterOrEqual,
        left: Box::new(support::query::column("orders.ordered_at")),
        right: Box::new(menura_model::expression::Expression::CurrentDate),
    }
}

#[test]
fn qry_2_a_filter_reading_the_current_date_without_reference_time_is_eqry_03() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        QueryTreeParts {
            filters: vec![support::query::filter("/query/filter", since_today())],
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model(), ANALYST),
        error(BindCode::MissingReferenceTime, "/query/filter")
    );
}

#[test]
fn qry_2_eqry_03_reports_the_filter_before_the_date_range_in_query_order() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        QueryTreeParts {
            filters: vec![support::query::filter("/query/filter", since_today())],
            date_range: Some(relative_range(
                RANGE,
                "orders.ordered_at",
                Grain::Day,
                7,
                None,
            )),
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model(), ANALYST),
        error(BindCode::MissingReferenceTime, "/query/filter")
    );
}

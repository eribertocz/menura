//! E2-5 row 12b, first part: `E2-01` (E2-3.4), `EQRY-09` (QRY-4.1), `EQRY-10` (QRY-4.5) and
//! `EQRY-11` (QRY-4.2), with their adjacent pairs, and the bound query they build.

#[cfg(test)]
mod support;

use menura_contracts::types::common::Grain;
use menura_e2_bind::BindCode;
use menura_representations::bound_query::BoundDimensionKind;
use menura_representations::names::OutputName;
use menura_representations::query_tree::{QueryDimension, QueryTreeParts};

use support::model::model;
use support::query::{absolute_range, date, dimension, metric, order_item, path, time_grain, tree};
use support::{bind_error, bind_query};

const ANALYST: &[&str] = &["analyst"];
const ADMIN: &[&str] = &["admin"];
const GRAINS: &str = "/query/nonportable_extensions/0/data/time_grains";
const RANGE: &str = "/query/nonportable_extensions/0/data/date_range";
const PATHS: &str = "/query/nonportable_extensions/0/data/paths";

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

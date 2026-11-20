//! E2-5 row 12b, second part: `EQRY-12` (QRY-4.3), `EQRY-03` for a relative range (QRY-2)
//! and `EQRY-04` (QRY-4.6), with their adjacent pairs, and the time of the bound query.

#[cfg(test)]
mod support;

use menura_contracts::types::common::Grain;
use menura_e2_bind::BindCode;
use menura_representations::query_tree::QueryTreeParts;
use menura_representations::time::{QueryTime, TimeZoneName};

use support::model::model;
use support::query::{dimension, limit, metric, order_item, path, relative_range, tree, tree_at};
use support::{bind_error, bind_query};

const ANALYST: &[&str] = &["analyst"];
const ADMIN: &[&str] = &["admin"];
const RANGE: &str = "/query/nonportable_extensions/0/data/date_range";
const PATHS: &str = "/query/nonportable_extensions/0/data/paths";
const LIMIT: &str = "/query/nonportable_extensions/0/data/limit";

fn error(code: BindCode, pointer: &str) -> (BindCode, String) {
    (code, pointer.to_owned())
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
        reference_date: None,
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

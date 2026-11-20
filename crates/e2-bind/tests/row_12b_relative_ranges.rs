//! E2-5 row 12b, relative ranges and the current time: `EQRY-03` for a filter that reads now
//! (QRY-2), the whole units of a relative range (QRY-4.2) and `E_INVALID_QUERY` outside the
//! years 0001–9999.

#[cfg(test)]
mod support;

use menura_contracts::types::common::Grain;
use menura_e2_bind::BindCode;
use menura_model::expression::DateValue;
use menura_representations::query_tree::QueryTreeParts;

use support::model::model;
use support::query::{date, dimension, limit, relative_range, tokyo_new_year, tree, tree_at};
use support::{bind_error, bind_query};

const ANALYST: &[&str] = &["analyst"];
const RANGE: &str = "/query/nonportable_extensions/0/data/date_range";

fn error(code: BindCode, pointer: &str) -> (BindCode, String) {
    (code, pointer.to_owned())
}

/// `CURRENT_DATE IS NOT NULL`: a boolean filter that reads the current date.
fn since_today() -> menura_model::expression::Expression {
    menura_model::expression::Expression::IsNull {
        operand: Box::new(menura_model::expression::Expression::CurrentDate),
        negated: true,
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

#[cfg(test)]
/// The bounds `bind` resolves for the relative range `unit`, `count` 2, at [`tokyo_new_year`].
fn resolved(unit: Grain, include_current: Option<bool>) -> (DateValue, DateValue) {
    let query = tree_at(
        vec![dimension(0, "orders.region")],
        vec![],
        tokyo_new_year(),
        QueryTreeParts {
            date_range: Some(relative_range(
                RANGE,
                "orders.ordered_at",
                unit,
                2,
                include_current,
            )),
            ..QueryTreeParts::default()
        },
    );
    let bound = bind_query(&query, &model(), ANALYST).expect("a bound query");
    let range = bound.date_range().expect("a date range");
    (range.from, range.to)
}

#[test]
fn qry_4_2_relative_units_including_the_current_one_count_from_the_local_date() {
    // The reference date in Tokyo is Friday 2027-01-01, while UTC is still on 2026-12-31.
    let cases = [
        (Grain::Day, (date(2026, 12, 31), date(2027, 1, 1))),
        (Grain::Week, (date(2026, 12, 21), date(2027, 1, 3))),
        (Grain::Month, (date(2026, 12, 1), date(2027, 1, 31))),
        (Grain::Quarter, (date(2026, 10, 1), date(2027, 3, 31))),
        (Grain::Year, (date(2026, 1, 1), date(2027, 12, 31))),
    ];
    for (unit, expected) in cases {
        assert_eq!(resolved(unit, Some(true)), expected, "{unit:?}");
        assert_eq!(
            resolved(unit, None),
            expected,
            "{unit:?}, include_current absent"
        );
    }
}

#[test]
fn qry_4_2_relative_units_without_the_current_one_end_with_the_previous_unit() {
    let cases = [
        (Grain::Day, (date(2026, 12, 30), date(2026, 12, 31))),
        (Grain::Week, (date(2026, 12, 14), date(2026, 12, 27))),
        (Grain::Month, (date(2026, 11, 1), date(2026, 12, 31))),
        (Grain::Quarter, (date(2026, 7, 1), date(2026, 12, 31))),
        (Grain::Year, (date(2025, 1, 1), date(2026, 12, 31))),
    ];
    for (unit, expected) in cases {
        assert_eq!(resolved(unit, Some(false)), expected, "{unit:?}");
    }
}

#[test]
fn qry_4_2_a_relative_range_outside_the_years_1_to_9999_is_invalid_query() {
    let query = tree_at(
        vec![dimension(0, "orders.region")],
        vec![],
        tokyo_new_year(),
        QueryTreeParts {
            date_range: Some(relative_range(
                RANGE,
                "orders.ordered_at",
                Grain::Year,
                100_000,
                None,
            )),
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model(), ANALYST),
        error(BindCode::InvalidQuery, RANGE)
    );
}

#[test]
fn qry_4_2_a_reference_time_whose_local_date_is_after_9999_is_invalid_query() {
    let mut time = tokyo_new_year();
    time.reference_date = None;
    let query = tree_at(
        vec![dimension(0, "orders.region")],
        vec![],
        time,
        QueryTreeParts {
            date_range: Some(relative_range(
                RANGE,
                "orders.ordered_at",
                Grain::Day,
                1,
                None,
            )),
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model(), ANALYST),
        error(BindCode::InvalidQuery, RANGE)
    );
}

#[test]
fn e2_5_rows_12b_invalid_query_comes_before_eqry_04() {
    let query = tree_at(
        vec![dimension(0, "orders.region")],
        vec![],
        tokyo_new_year(),
        QueryTreeParts {
            date_range: Some(relative_range(
                RANGE,
                "orders.ordered_at",
                Grain::Year,
                100_000,
                None,
            )),
            limit: Some(limit(10)),
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model(), ANALYST),
        error(BindCode::InvalidQuery, RANGE)
    );
}

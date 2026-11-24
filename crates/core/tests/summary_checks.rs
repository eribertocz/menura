//! `integrity_checks` over the `chk-summaries` conformance model: the `summary` check of each
//! summary and its build query (SUM-5, CHK-5, CHK-6.6, CHK-7).

#![cfg(test)]

use menura_contracts::types::descriptions::{
    Binding, BuildQuery, CheckKind, CheckParameter, IntegrityCheck,
};
use menura_core::{Dialect, Handle, LoadOptions, integrity_checks, load};

const MODEL: &str = include_str!("../../../conformance/models/chk-summaries/model.yaml");
const CATALOG_SCHEMA: &str =
    include_str!("../../../conformance/models/chk-summaries/catalog-schema.json");

fn handle() -> Handle {
    load(
        MODEL.as_bytes(),
        CATALOG_SCHEMA.as_bytes(),
        None,
        &LoadOptions::default(),
    )
    .unwrap()
}

fn check(id: &str) -> IntegrityCheck {
    integrity_checks(&handle(), Dialect::Duckdb)
        .unwrap()
        .checks
        .into_iter()
        .find(|check| check.check_id == id)
        .unwrap()
}

fn build(id: &str) -> BuildQuery {
    check(id).build.unwrap()
}

fn bindings(parameters: &[CheckParameter]) -> Vec<(u64, Binding)> {
    parameters
        .iter()
        .map(|parameter| match parameter {
            CheckParameter::Bound(slot) => (slot.position, slot.binding),
            CheckParameter::Fixed(fixed) => panic!("a fixed parameter at {}", fixed.position),
        })
        .collect()
}

const MONTHLY: &str = "summary:ORDERS_MONTHLY";
const DAILY: &str = "summary:ORDERS_DAILY_LOCAL";

#[test]
fn sum_5_each_summary_has_one_summary_check() {
    let document = integrity_checks(&handle(), Dialect::Duckdb).unwrap();
    let summaries: Vec<(String, CheckKind, String)> = document
        .checks
        .into_iter()
        .filter(|check| check.kind == CheckKind::Summary)
        .map(|check| (check.check_id, check.kind, check.target))
        .collect();
    assert_eq!(
        summaries,
        [
            (
                DAILY.to_owned(),
                CheckKind::Summary,
                "ORDERS_DAILY_LOCAL".to_owned()
            ),
            (
                MONTHLY.to_owned(),
                CheckKind::Summary,
                "ORDERS_MONTHLY".to_owned()
            ),
        ]
    );
}

#[test]
fn chk_3_a_summary_check_carries_the_summary_definition_hash() {
    let handle = handle();
    let summary = handle
        .model()
        .parts()
        .summaries
        .iter()
        .find(|summary| summary.dataset.as_str() == "ORDERS_MONTHLY")
        .unwrap();
    assert_eq!(check(MONTHLY).definition_hash, summary.definition_hash);
}

#[test]
fn chk_2_4_a_summary_check_reads_the_base_and_the_summary() {
    assert_eq!(
        check(MONTHLY).sources,
        ["shop.orders", "shop.orders_monthly"]
    );
}

#[test]
fn chk_5_4_only_a_summary_check_carries_a_build_query() {
    let document = integrity_checks(&handle(), Dialect::Duckdb).unwrap();
    assert!(
        document
            .checks
            .iter()
            .all(|check| check.build.is_some() == (check.kind == CheckKind::Summary))
    );
}

#[test]
fn chk_5_5_build_columns_are_the_catalog_columns_in_field_order() {
    // The fields of the internal model are ordered by name (E0H-1.4).
    assert_eq!(
        build(MONTHLY).columns,
        ["month_start", "order_count", "region", "total_amount"]
    );
    assert_eq!(build(DAILY).columns, ["day_start", "large_amount"]);
}

#[test]
fn chk_5_5_the_build_query_groups_by_truncated_dimensions_and_aggregates_measures() {
    let sql = build(MONTHLY).sql;
    assert!(sql.contains("date_trunc('month'"), "{sql}");
    assert!(sql.contains("SUM("), "{sql}");
    assert!(sql.contains("COUNT("), "{sql}");
    assert!(sql.contains("GROUP BY 1, 2"), "{sql}");
    assert!(!sql.contains("orders_monthly"), "{sql}");
}

#[test]
fn chk_5_1_the_check_recomputes_the_base_exactly_as_the_build_query() {
    let build = build(MONTHLY).sql;
    let check = check(MONTHLY).sql;
    let aggregate_step = |sql: &str| {
        let start = sql.find("_aggregate\" AS (").unwrap();
        let end = start + sql[start..].find("\n  )").unwrap();
        sql[start..end].to_owned()
    };
    assert_eq!(aggregate_step(&build), aggregate_step(&check));
}

#[test]
fn chk_6_6_the_check_full_joins_both_sides_on_every_dimension_without_casts() {
    let sql = check(MONTHLY).sql;
    assert!(sql.contains("FULL OUTER JOIN"), "{sql}");
    assert_eq!(sql.matches("IS NOT DISTINCT FROM").count(), 2, "{sql}");
    for violation in ["missing_in_summary", "missing_in_base", "measure_differs"] {
        assert!(sql.contains(&format!("'{violation}'")), "{sql}");
    }
}

#[test]
fn chk_6_6_check_columns_are_dimensions_then_measure_pairs_then_violation() {
    let sql = check(MONTHLY).sql;
    let last = sql
        .lines()
        .skip_while(|line| *line != "SELECT")
        .collect::<Vec<_>>();
    assert_eq!(
        last,
        [
            "SELECT",
            "  \"MONTH\",",
            "  \"REGION\",",
            "  \"base.ORDER_COUNT\",",
            "  \"summary.ORDER_COUNT\",",
            "  \"base.TOTAL_AMOUNT\",",
            "  \"summary.TOTAL_AMOUNT\",",
            "  \"violation\"",
            "FROM \"menura_8_filter\"",
        ]
    );
}

#[test]
fn chk_7_4_a_date_summary_binds_only_the_period_slots() {
    let check = check(MONTHLY);
    assert_eq!(
        bindings(&check.parameters),
        [(1, Binding::PeriodFrom), (2, Binding::PeriodTo)]
    );
    assert_eq!(
        bindings(&check.build.unwrap().parameters),
        [(1, Binding::PeriodFrom), (2, Binding::PeriodTo)]
    );
}

#[test]
fn chk_7_3_a_summary_of_a_timestamp_with_time_zone_binds_the_time_zone() {
    let check = check(DAILY);
    let expected = [
        (1, Binding::TimeZone),
        (2, Binding::PeriodFrom),
        (3, Binding::PeriodTo),
    ];
    assert_eq!(bindings(&check.parameters), expected);
    assert_eq!(bindings(&check.build.unwrap().parameters), expected);
}

#[test]
fn chk_7_2_each_slot_appears_once_and_is_shared_by_every_use() {
    let check = check(DAILY);
    assert!(
        check.sql.matches("CAST($1 AS VARCHAR)").count() >= 2,
        "{}",
        check.sql
    );
    assert!(
        check.sql.matches("CAST($2 AS VARCHAR)").count() >= 2,
        "{}",
        check.sql
    );
}

/// The positions of the placeholders of `sql`, in order of first appearance.
fn first_appearances(sql: &str) -> Vec<u64> {
    let mut positions = Vec::new();
    for (index, _) in sql.match_indices('$') {
        let digits: String = sql[index + 1..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        let position: u64 = digits.parse().unwrap();
        if !positions.contains(&position) {
            positions.push(position);
        }
    }
    positions
}

#[test]
fn e6_4_2_slot_positions_follow_their_first_appearance_in_the_sql() {
    for id in [MONTHLY, DAILY] {
        let check = check(id);
        let build = check.build.clone().unwrap();
        for (sql, count) in [
            (&check.sql, check.parameters.len()),
            (&build.sql, build.parameters.len()),
        ] {
            let expected: Vec<u64> = (1..=u64::try_from(count).unwrap()).collect();
            assert_eq!(first_appearances(sql), expected, "{id}: {sql}");
        }
    }
}

#[test]
fn chk_2_5_a_summary_check_has_no_limit() {
    assert!(!check(MONTHLY).sql.contains("LIMIT"));
    assert!(!build(MONTHLY).sql.contains("LIMIT"));
}

//! `integrity_checks` over the `chk-populations` conformance model: the `population` checks of
//! each mode and the `semi_additive` checks of a time `over` and of a field `over` (E0H-4.3,
//! CHK-6.3, CHK-6.4, CHK-6.5, CHK-7.3, POP-5, ADD-6).

#![cfg(test)]

use menura_contracts::types::descriptions::{
    Binding, BoundParameter, CheckKind, CheckParameter, IntegrityCheck, TextLogicalType,
};
use menura_core::{Dialect, Handle, IntegrityChecksDocument, LoadOptions, integrity_checks, load};

const MODEL: &str = include_str!("../../../conformance/models/chk-populations/model.yaml");
const CATALOG_SCHEMA: &str =
    include_str!("../../../conformance/models/chk-populations/catalog-schema.json");

fn handle() -> Handle {
    load(
        MODEL.as_bytes(),
        CATALOG_SCHEMA.as_bytes(),
        None,
        &LoadOptions::default(),
    )
    .unwrap()
}

fn document() -> IntegrityChecksDocument {
    integrity_checks(&handle(), Dialect::Duckdb).unwrap()
}

fn check(id: &str) -> IntegrityCheck {
    document()
        .checks
        .into_iter()
        .find(|check| check.check_id == id)
        .unwrap()
}

/// The names of the columns the last `SELECT` of `sql` returns, in order.
fn result_columns(sql: &str) -> Vec<String> {
    let last = sql.rsplit("\nSELECT\n").next().unwrap();
    last.lines()
        .take_while(|line| line.starts_with("  "))
        .map(|line| {
            let item = line.trim().trim_end_matches(',');
            let name = item.rsplit(" AS ").next().unwrap();
            name.trim_matches('"').to_owned()
        })
        .collect()
}

#[test]
fn e0h_4_3_population_and_semi_additive_checks_sorted_by_check_id() {
    let ids: Vec<String> = document()
        .checks
        .into_iter()
        .map(|check| check.check_id)
        .collect();
    assert_eq!(
        ids,
        [
            "population:MEMBERS",
            "population:MONTHLY_ROSTER",
            "population:STAMPED_MEMBERS",
            "semi_additive:ROSTER_BALANCE",
            "semi_additive:SNAPSHOT_BALANCE",
        ]
    );
}

#[test]
fn chk_2_each_check_has_its_kind_target_and_recorded_hash() {
    let handle = handle();
    let document = integrity_checks(&handle, Dialect::Duckdb).unwrap();
    for generated in &document.checks {
        let recorded = handle
            .model()
            .parts()
            .checks
            .iter()
            .find(|recorded| recorded.check_id == generated.check_id)
            .unwrap();
        assert_eq!(generated.definition_hash, recorded.definition_hash);
        assert!(generated.build.is_none());
    }
    assert_eq!(check("population:MEMBERS").kind, CheckKind::Population);
    assert_eq!(check("population:MEMBERS").target, "MEMBERS");
    let semi_additive = check("semi_additive:ROSTER_BALANCE");
    assert_eq!(semi_additive.kind, CheckKind::SemiAdditive);
    assert_eq!(semi_additive.target, "ROSTER_BALANCE");
}

#[test]
fn chk_2_4_sources_are_the_relations_read_sorted() {
    assert_eq!(
        check("population:MEMBERS").sources,
        ["shop.memberships"]
    );
    assert_eq!(
        check("semi_additive:ROSTER_BALANCE").sources,
        ["shop.balances", "shop.roster"]
    );
    assert_eq!(
        check("semi_additive:SNAPSHOT_BALANCE").sources,
        ["shop.balances"]
    );
}

#[test]
fn chk_6_3_validity_columns_are_entity_valid_from_valid_to_then_violation() {
    let sql = check("population:MEMBERS").sql;
    assert_eq!(
        result_columns(&sql),
        ["CUSTOMER_ID", "VALID_FROM", "VALID_TO", "violation"]
    );
    assert!(sql.contains("'invalid_interval'"), "{sql}");
    assert!(sql.contains("'overlap'"), "{sql}");
}

#[test]
fn chk_6_4_periodic_columns_are_entity_period_then_row_count() {
    let sql = check("population:MONTHLY_ROSTER").sql;
    assert_eq!(
        result_columns(&sql),
        ["CUSTOMER_ID", "PERIOD_AT", "row_count"]
    );
    assert!(sql.contains("date_trunc('month'"), "{sql}");
}

#[test]
fn chk_6_5_semi_additive_columns_are_entity_over_then_violation() {
    let sql = check("semi_additive:ROSTER_BALANCE").sql;
    assert_eq!(result_columns(&sql), ["CUSTOMER_ID", "PERIOD", "violation"]);
    for violation in [
        "'duplicate_snapshot'",
        "'gap'",
        "'missing_entity'",
        "'unexpected_entity'",
    ] {
        assert!(sql.contains(violation), "{violation}: {sql}");
    }
}

#[test]
fn chk_6_5_a_field_over_has_only_duplicate_snapshot() {
    let sql = check("semi_additive:SNAPSHOT_BALANCE").sql;
    assert_eq!(
        result_columns(&sql),
        ["CUSTOMER_ID", "SNAPSHOT", "violation"]
    );
    assert!(sql.contains("'duplicate_snapshot'"), "{sql}");
    for violation in ["'gap'", "'missing_entity'", "'unexpected_entity'"] {
        assert!(!sql.contains(violation), "{violation}: {sql}");
    }
}

#[test]
fn chk_7_3_a_timestamp_with_time_zone_population_has_one_time_zone_slot() {
    let check = check("population:STAMPED_MEMBERS");
    assert_eq!(
        check.parameters,
        [CheckParameter::Bound(BoundParameter {
            position: 1,
            logical_type: TextLogicalType,
            binding: Binding::TimeZone,
        })]
    );
    assert!(
        check.sql.contains("AT TIME ZONE CAST($1 AS VARCHAR)"),
        "{}",
        check.sql
    );
}

#[test]
fn chk_7_3_date_fields_need_no_time_zone_slot() {
    for id in [
        "population:MEMBERS",
        "population:MONTHLY_ROSTER",
        "semi_additive:ROSTER_BALANCE",
        "semi_additive:SNAPSHOT_BALANCE",
    ] {
        let check = check(id);
        assert!(check.parameters.is_empty(), "{id}: {:?}", check.parameters);
        assert!(!check.sql.contains("AT TIME ZONE"), "{id}: {}", check.sql);
    }
}

#[test]
fn chk_2_5_a_check_query_has_no_limit() {
    for check in document().checks {
        assert!(!check.sql.contains("LIMIT"), "{}", check.sql);
    }
}

#[test]
fn chk_4_2_check_queries_come_from_the_generator_of_e6() {
    for check in document().checks {
        assert!(
            check.sql.starts_with("WITH\n  \"menura_1_scan\" AS ("),
            "{}",
            check.sql
        );
    }
}

#[test]
fn chk_2_the_same_model_gives_the_same_document() {
    assert_eq!(document(), document());
}

#[test]
fn chk_6_5_a_metric_reading_over_or_entity_from_a_reached_dataset_has_no_check() {
    let handle = load(
        include_bytes!("fixtures/reached-snapshot/model.yaml"),
        include_bytes!("fixtures/reached-snapshot/catalog-schema.json"),
        None,
        &LoadOptions::default(),
    )
    .unwrap();
    assert!(
        handle
            .model()
            .parts()
            .checks
            .iter()
            .any(|recorded| recorded.check_id == "semi_additive:REACHED_BALANCE")
    );
    let ids: Vec<String> = integrity_checks(&handle, Dialect::Duckdb)
        .unwrap()
        .checks
        .into_iter()
        .map(|check| check.check_id)
        .collect();
    assert_eq!(
        ids,
        [
            "key_uniqueness:BALANCES_CUSTOMER",
            "orphan_keys:BALANCES_CUSTOMER",
            "semi_additive:HOME_BALANCE",
        ]
    );
}

#[test]
fn chk_6_3_rows_are_numbered_in_the_order_of_every_scanned_column() {
    let sql = check("population:MEMBERS").sql;
    assert!(
        sql.contains(
            "row_number() OVER (ORDER BY \"CUSTOMER_ID\", \"VALID_FROM\", \"VALID_TO\") AS \"menura.row\""
        ),
        "{sql}"
    );
}

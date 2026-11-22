//! Executes the `population` and `semi_additive` checks of `integrity_checks` on `DuckDB`
//! (E6-5.4): over the violating data of the `chk-populations` model each check returns exactly
//! its violations (CHK-2.1, CHK-6.3, CHK-6.4, CHK-6.5, POP-5, ADD-6), a `timestamp with time
//! zone` reads its days in the bound time zone (CHK-7.3), and once the violations are removed
//! every check returns no row.

#![cfg(test)]

use std::path::{Path, PathBuf};

use duckdb::Connection;
use menura_conformance_runner::database::prepare_database;
use menura_conformance_runner::fixture::ModelFixture;
use menura_contracts::types::descriptions::{Binding, CheckParameter};
use menura_core::{Dialect, IntegrityChecksDocument, LoadOptions, integrity_checks, load};

fn model_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/models/chk-populations")
}

fn document() -> IntegrityChecksDocument {
    let model = include_bytes!("../../../conformance/models/chk-populations/model.yaml");
    let catalog_schema =
        include_bytes!("../../../conformance/models/chk-populations/catalog-schema.json");
    let handle = load(model, catalog_schema, None, &LoadOptions::default()).unwrap();
    integrity_checks(&handle, Dialect::Duckdb).unwrap()
}

fn database() -> Connection {
    prepare_database(&ModelFixture::read(&model_directory()).unwrap()).unwrap()
}

/// The rows of the check `id` with the time zone `time_zone` bound to its slot, each value as
/// text (`NULL` as `None`), sorted.
fn violations(
    connection: &Connection,
    document: &IntegrityChecksDocument,
    id: &str,
    time_zone: &str,
) -> Vec<Vec<Option<String>>> {
    let check = document
        .checks
        .iter()
        .find(|check| check.check_id == id)
        .unwrap();
    let values: Vec<String> = check
        .parameters
        .iter()
        .map(|parameter| match parameter {
            CheckParameter::Bound(slot) => {
                assert_eq!(slot.binding, Binding::TimeZone);
                time_zone.to_owned()
            }
            CheckParameter::Fixed(fixed) => panic!("unexpected fixed parameter {fixed:?}"),
        })
        .collect();
    let text = format!(
        "SELECT CAST(COLUMNS(*) AS VARCHAR) FROM ({}) AS \"check\"",
        check.sql
    );
    let mut statement = connection.prepare(&text).unwrap();
    let mut rows = statement
        .query(duckdb::params_from_iter(values.iter()))
        .unwrap();
    let mut found = Vec::new();
    while let Some(row) = rows.next().unwrap() {
        let width = row.as_ref().column_count();
        found.push(
            (0..width)
                .map(|index| row.get::<_, Option<String>>(index).unwrap())
                .collect(),
        );
    }
    found.sort();
    found
}

fn row(values: &[Option<&str>]) -> Vec<Option<String>> {
    values
        .iter()
        .map(|value| value.map(str::to_owned))
        .collect()
}

#[test]
fn pop_5_chk_6_3_validity_returns_invalid_intervals_and_each_overlapping_row() {
    let document = document();
    let connection = database();
    assert_eq!(
        violations(&connection, &document, "population:MEMBERS", "UTC"),
        [
            row(&[
                Some("1"),
                Some("2026-01-01"),
                Some("2026-01-31"),
                Some("overlap")
            ]),
            row(&[
                Some("1"),
                Some("2026-01-31"),
                Some("2026-02-28"),
                Some("overlap")
            ]),
            row(&[
                Some("2"),
                Some("2026-03-10"),
                Some("2026-03-01"),
                Some("invalid_interval")
            ]),
            row(&[Some("5"), Some("2026-01-01"), None, Some("overlap")]),
            row(&[Some("5"), Some("2026-01-01"), None, Some("overlap")]),
        ]
    );
}

#[test]
fn chk_6_3_equal_rows_among_many_rows_are_each_an_overlap_and_no_row_overlaps_itself() {
    let document = document();
    let connection = database();
    connection
        .execute_batch(
            "DELETE FROM shop.main.memberships;
             INSERT INTO shop.main.memberships
               SELECT i, DATE '2026-01-01', DATE '2026-01-31' FROM range(10, 300010) AS t(i);
             INSERT INTO shop.main.memberships VALUES
               (6, DATE '2026-02-01', NULL),
               (6, DATE '2026-02-01', NULL),
               (6, DATE '2026-02-01', NULL),
               (7, DATE '2026-02-01', DATE '2026-02-01');",
        )
        .unwrap();
    assert_eq!(
        violations(&connection, &document, "population:MEMBERS", "UTC"),
        [
            row(&[Some("6"), Some("2026-02-01"), None, Some("overlap")]),
            row(&[Some("6"), Some("2026-02-01"), None, Some("overlap")]),
            row(&[Some("6"), Some("2026-02-01"), None, Some("overlap")]),
        ]
    );
}

#[test]
fn chk_7_3_validity_reads_days_in_the_bound_time_zone() {
    let document = document();
    let connection = database();
    assert!(violations(&connection, &document, "population:STAMPED_MEMBERS", "UTC").is_empty());
    let found = violations(
        &connection,
        &document,
        "population:STAMPED_MEMBERS",
        "America/La_Paz",
    );
    assert_eq!(found.len(), 2, "{found:?}");
    for violation in found {
        assert_eq!(violation[0].as_deref(), Some("1"));
        assert_eq!(violation[3].as_deref(), Some("overlap"));
    }
}

#[test]
fn pop_5_chk_6_4_periodic_returns_each_entity_and_period_with_more_than_one_row() {
    let document = document();
    let connection = database();
    assert_eq!(
        violations(&connection, &document, "population:MONTHLY_ROSTER", "UTC"),
        [row(&[Some("1"), Some("2026-03-01"), Some("2")])]
    );
}

#[test]
fn add_6_chk_6_5_a_time_over_returns_each_add_6_condition() {
    let document = document();
    let connection = database();
    assert_eq!(
        violations(
            &connection,
            &document,
            "semi_additive:ROSTER_BALANCE",
            "UTC"
        ),
        [
            row(&[None, Some("2026-02-01"), Some("gap")]),
            row(&[Some("1"), Some("2026-01-01"), Some("duplicate_snapshot")]),
            row(&[Some("1"), Some("2026-01-01"), Some("unexpected_entity")]),
            row(&[Some("2"), Some("2026-03-01"), Some("missing_entity")]),
            row(&[Some("4"), Some("2026-03-01"), Some("unexpected_entity")]),
        ]
    );
}

#[test]
fn chk_6_5_a_field_over_returns_only_duplicate_snapshots() {
    let document = document();
    let connection = database();
    assert_eq!(
        violations(
            &connection,
            &document,
            "semi_additive:SNAPSHOT_BALANCE",
            "UTC"
        ),
        [row(&[Some("1"), Some("1"), Some("duplicate_snapshot")])]
    );
}

#[test]
fn chk_2_1_checks_over_data_without_violations_return_no_row() {
    let document = document();
    let connection = database();
    connection
        .execute_batch(
            "DELETE FROM shop.main.memberships WHERE customer_id IN (1, 2, 5);
             DELETE FROM shop.main.stamped_memberships WHERE valid_to IS NULL;
             DELETE FROM shop.main.roster WHERE period_at = DATE '2026-03-15';
             INSERT INTO shop.main.roster VALUES (4, DATE '2026-03-01');
             DELETE FROM shop.main.balances WHERE period < DATE '2026-03-01';
             INSERT INTO shop.main.balances VALUES (2, DATE '2026-03-31', 6, 80);",
        )
        .unwrap();
    for check in &document.checks {
        for time_zone in ["UTC", "America/La_Paz"] {
            assert!(
                violations(&connection, &document, &check.check_id, time_zone).is_empty(),
                "{} in {time_zone}",
                check.check_id
            );
        }
    }
}

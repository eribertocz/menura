//! Executes the `population` checks of the `val-validity-checks` model on `DuckDB` (E6-5.4): an
//! exclusive `valid_to` compares values, so a row that starts where it ends is
//! `invalid_interval` and two rows that meet are no overlap (VAL-1.4), its check binds no
//! `time_zone` (CHK-7.3), and a `valid_to` equal to `open_end` is open-ended (VAL-1.2).

#![cfg(test)]

use std::path::{Path, PathBuf};

use duckdb::Connection;
use menura_conformance_runner::database::prepare_database;
use menura_conformance_runner::fixture::ModelFixture;
use menura_core::{Dialect, IntegrityChecksDocument, LoadOptions, integrity_checks, load};

fn model_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/models/val-validity-checks")
}

fn document() -> IntegrityChecksDocument {
    let model = include_bytes!("../../../conformance/models/val-validity-checks/model.yaml");
    let catalog_schema =
        include_bytes!("../../../conformance/models/val-validity-checks/catalog-schema.json");
    let handle = load(model, catalog_schema, None, &LoadOptions::default()).unwrap();
    integrity_checks(&handle, Dialect::Duckdb).unwrap()
}

fn database() -> Connection {
    prepare_database(&ModelFixture::read(&model_directory()).unwrap()).unwrap()
}

/// The rows of the check `id`, which has no parameter, each value as text (`NULL` as
/// `None`), sorted.
fn violations(
    connection: &Connection,
    document: &IntegrityChecksDocument,
    id: &str,
) -> Vec<Vec<Option<String>>> {
    let check = document
        .checks
        .iter()
        .find(|check| check.check_id == id)
        .unwrap();
    assert!(check.parameters.is_empty(), "{:?}", check.parameters);
    let text = format!(
        "SELECT CAST(COLUMNS(*) AS VARCHAR) FROM ({}) AS \"check\"",
        check.sql
    );
    let mut statement = connection.prepare(&text).unwrap();
    let mut rows = statement.query([]).unwrap();
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
fn val_1_4_an_exclusive_date_range_compares_values() {
    let document = document();
    let connection = database();
    assert_eq!(
        violations(&connection, &document, "population:DATE_EXCLUSIVE"),
        [
            row(&[
                Some("2"),
                Some("2026-01-01"),
                Some("2026-01-01"),
                Some("invalid_interval")
            ]),
            row(&[
                Some("3"),
                Some("2026-01-01"),
                Some("2026-02-02"),
                Some("overlap")
            ]),
            row(&[Some("3"), Some("2026-02-01"), None, Some("overlap")]),
        ]
    );
}

#[test]
fn val_1_4_chk_7_3_an_exclusive_timestamp_range_binds_no_time_zone() {
    let document = document();
    let connection = database();
    let found: Vec<(Option<String>, Option<String>)> =
        violations(&connection, &document, "population:STAMP_EXCLUSIVE")
            .into_iter()
            .map(|violation| (violation[0].clone(), violation[3].clone()))
            .collect();
    let pair =
        |customer: &str, violation: &str| (Some(customer.to_owned()), Some(violation.to_owned()));
    assert_eq!(
        found,
        [
            pair("2", "invalid_interval"),
            pair("3", "overlap"),
            pair("3", "overlap"),
        ]
    );
}

#[test]
fn val_1_2_a_valid_to_equal_to_open_end_is_open_ended_in_the_check() {
    let document = document();
    let connection = database();
    assert_eq!(
        violations(&connection, &document, "population:DATE_OPEN_END"),
        [
            row(&[
                Some("1"),
                Some("2026-01-01"),
                Some("2000-01-01"),
                Some("overlap")
            ]),
            row(&[
                Some("1"),
                Some("2026-06-01"),
                Some("2026-06-30"),
                Some("overlap")
            ]),
            row(&[
                Some("2"),
                Some("2026-03-01"),
                Some("2026-02-01"),
                Some("invalid_interval")
            ]),
        ]
    );
}

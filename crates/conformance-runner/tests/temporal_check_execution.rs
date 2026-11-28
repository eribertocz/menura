//! Executes the `temporal_orphans` checks of the `val-temporal` model on `DuckDB` (E6-5.4,
//! VAL-2.16a): a sale whose time is where an exclusive version ends, with no version starting
//! there, is a violation; a time after an `open_end` is covered; a NULL `from_time` or key is
//! never a violation; a time before the first version, or with no version of its entity, is a
//! violation. The check binds no `time_zone` (CHK-7.3).

#![cfg(test)]

use std::path::{Path, PathBuf};

use duckdb::Connection;
use menura_conformance_runner::database::prepare_database;
use menura_conformance_runner::fixture::ModelFixture;
use menura_core::{Dialect, IntegrityChecksDocument, LoadOptions, integrity_checks, load};

fn model_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/models/val-temporal")
}

fn document() -> IntegrityChecksDocument {
    let model = include_bytes!("../../../conformance/models/val-temporal/model.yaml");
    let catalog_schema =
        include_bytes!("../../../conformance/models/val-temporal/catalog-schema.json");
    let handle = load(model, catalog_schema, None, &LoadOptions::default()).unwrap();
    integrity_checks(&handle, Dialect::Duckdb).unwrap()
}

/// The fixture database, with the session in UTC so that timestamps read back in UTC; the
/// checks compare values and read no time zone (VAL-2.9).
fn database() -> Connection {
    let connection = prepare_database(&ModelFixture::read(&model_directory()).unwrap()).unwrap();
    connection.execute_batch("SET TimeZone = 'UTC';").unwrap();
    connection
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

fn row(values: &[&str]) -> Vec<Option<String>> {
    values
        .iter()
        .map(|value| Some((*value).to_owned()))
        .collect()
}

#[test]
fn val_2_16a_an_exclusive_date_history_with_an_open_end() {
    let document = document();
    let connection = database();
    assert_eq!(
        violations(&connection, &document, "temporal_orphans:SALE_CUSTOMER"),
        [
            row(&["2", "2026-02-15"]),
            row(&["2", "2026-04-01"]),
            row(&["9", "2026-01-01"]),
        ]
    );
}

#[test]
fn val_2_16a_chk_7_3_an_inclusive_timestamp_history_binds_no_time_zone() {
    let document = document();
    let connection = database();
    assert_eq!(
        violations(&connection, &document, "temporal_orphans:VISIT_TIER"),
        [
            row(&["1", "2025-12-31 23:00:00+00"]),
            row(&["1", "2026-01-31 23:59:59.5+00"]),
        ]
    );
}

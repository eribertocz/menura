//! Executes the relationship checks of `integrity_checks` on `DuckDB` (E6-5.4): over the
//! violating data of the `chk-relationships` model each check returns exactly its violations
//! (CHK-2.1, CHK-6.1, CHK-6.2), and once the violations are removed every check returns no row.

#![cfg(test)]

use std::path::{Path, PathBuf};

use duckdb::Connection;
use menura_conformance_runner::database::prepare_database;
use menura_conformance_runner::fixture::ModelFixture;
use menura_core::{Dialect, IntegrityChecksDocument, LoadOptions, integrity_checks, load};

fn model_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/models/chk-relationships")
}

fn document() -> IntegrityChecksDocument {
    let model = include_bytes!("../../../conformance/models/chk-relationships/model.yaml");
    let catalog_schema =
        include_bytes!("../../../conformance/models/chk-relationships/catalog-schema.json");
    let handle = load(model, catalog_schema, None, &LoadOptions::default()).unwrap();
    integrity_checks(&handle, Dialect::Duckdb).unwrap()
}

fn database() -> Connection {
    prepare_database(&ModelFixture::read(&model_directory()).unwrap()).unwrap()
}

/// The rows of the check `id`, each value as text (`NULL` as `None`), sorted.
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
    assert!(check.parameters.is_empty());
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
fn chk_6_1_key_uniqueness_returns_each_repeated_complete_key_with_its_count() {
    let document = document();
    let connection = database();
    assert_eq!(
        violations(&connection, &document, "key_uniqueness:INVOICES_CUSTOMER"),
        [row(&["2", "2"])]
    );
}

#[test]
fn chk_6_2_orphan_keys_returns_each_unmatched_row_with_complete_keys() {
    let document = document();
    let connection = database();
    assert_eq!(
        violations(&connection, &document, "orphan_keys:INVOICES_CUSTOMER"),
        [row(&["3"]), row(&["3"])]
    );
    assert_eq!(
        violations(&connection, &document, "orphan_keys:INVOICES_STORE"),
        [row(&["CBB"])]
    );
}

#[test]
fn chk_2_1_a_check_over_data_without_violations_returns_no_row() {
    let document = document();
    let connection = database();
    assert!(violations(&connection, &document, "key_uniqueness:INVOICES_STORE").is_empty());
    connection
        .execute_batch(
            "DELETE FROM shop.main.customers WHERE region = 'EAST';
             DELETE FROM shop.main.invoices WHERE customer_id = 3;",
        )
        .unwrap();
    for check in &document.checks {
        assert!(
            violations(&connection, &document, &check.check_id).is_empty(),
            "{}",
            check.check_id
        );
    }
}

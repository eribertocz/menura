//! Executes the summary checks of `integrity_checks` on `DuckDB` (E6-5.4): the build query of
//! a summary builds its rows from the base in the bound period range (CHK-5.1, CHK-5.5,
//! CHK-7.4), the summary it built passes its check, and a corrupted row fails it (CHK-6.6).

#![cfg(test)]

use std::path::{Path, PathBuf};

use duckdb::{Connection, params_from_iter};
use menura_conformance_runner::database::prepare_database;
use menura_conformance_runner::fixture::ModelFixture;
use menura_core::{Dialect, IntegrityChecksDocument, LoadOptions, integrity_checks, load};

type Rows = Vec<Vec<Option<String>>>;

const MONTHLY: &str = "summary:ORDERS_MONTHLY";
const DAILY: &str = "summary:ORDERS_DAILY_LOCAL";

fn model_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/models/chk-summaries")
}

fn document() -> IntegrityChecksDocument {
    let model = include_bytes!("../../../conformance/models/chk-summaries/model.yaml");
    let catalog_schema =
        include_bytes!("../../../conformance/models/chk-summaries/catalog-schema.json");
    let handle = load(model, catalog_schema, None, &LoadOptions::default()).unwrap();
    integrity_checks(&handle, Dialect::Duckdb).unwrap()
}

fn database() -> Connection {
    prepare_database(&ModelFixture::read(&model_directory()).unwrap()).unwrap()
}

/// The SQL of the check `id`, or of its build query when `build`.
fn sql(document: &IntegrityChecksDocument, id: &str, build: bool) -> String {
    let check = document
        .checks
        .iter()
        .find(|check| check.check_id == id)
        .unwrap();
    if build {
        check.build.as_ref().unwrap().sql.clone()
    } else {
        check.sql.clone()
    }
}

/// The rows of `sql` with the slots bound to `values` in position order, each value as text
/// (`NULL` as `None`), sorted.
fn rows(connection: &Connection, sql: &str, values: &[&str]) -> Rows {
    let text = format!("SELECT CAST(COLUMNS(*) AS VARCHAR) FROM ({sql}) AS \"result\"");
    let mut statement = connection.prepare(&text).unwrap();
    let mut rows = statement.query(params_from_iter(values.iter())).unwrap();
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

/// Inserts the rows of the build query `build`, bound to `values`, into `table`.
fn build_into(connection: &Connection, table: &str, build: &str, values: &[&str]) {
    let text = format!("INSERT INTO {table} SELECT * FROM ({build}) AS \"built\"");
    connection
        .execute(&text, params_from_iter(values.iter()))
        .unwrap();
}

fn row(values: &[Option<&str>]) -> Vec<Option<String>> {
    values
        .iter()
        .map(|value| value.map(str::to_owned))
        .collect()
}

/// The `violation` column of each row, sorted.
fn violations(found: &Rows) -> Vec<String> {
    let mut violations: Vec<String> = found
        .iter()
        .map(|row| row.last().cloned().flatten().unwrap())
        .collect();
    violations.sort();
    violations
}

const OCTOBER_TO_NOVEMBER: [&str; 2] = ["2026-10", "2026-11"];

#[test]
fn chk_5_5_the_build_query_returns_one_row_per_group_in_the_bound_range() {
    let document = document();
    let connection = database();
    assert_eq!(
        rows(
            &connection,
            &sql(&document, MONTHLY, true),
            &OCTOBER_TO_NOVEMBER
        ),
        [
            row(&[Some("2026-10-01"), Some("2"), Some("NORTH"), Some("110.00")]),
            row(&[Some("2026-11-01"), Some("1"), None, Some("30.00")]),
            row(&[Some("2026-11-01"), Some("1"), Some("SOUTH"), Some("50.00")]),
        ]
    );
}

#[test]
fn chk_6_6_a_built_summary_passes_its_check_and_corrupted_rows_fail_it() {
    let document = document();
    let connection = database();
    let check = sql(&document, MONTHLY, false);
    build_into(
        &connection,
        "shop.main.orders_monthly (month_start, order_count, region, total_amount)",
        &sql(&document, MONTHLY, true),
        &OCTOBER_TO_NOVEMBER,
    );
    // The December row is wrong, but outside the bound range (CHK-7.4).
    assert!(rows(&connection, &check, &OCTOBER_TO_NOVEMBER).is_empty());
    connection
        .execute_batch(
            "UPDATE shop.main.orders_monthly SET total_amount = 111.00 WHERE region = 'NORTH';
             DELETE FROM shop.main.orders_monthly WHERE region = 'SOUTH' AND month_start = DATE '2026-11-01';
             INSERT INTO shop.main.orders_monthly VALUES ('EAST', DATE '2026-10-01', 1.00, 1);",
        )
        .unwrap();
    assert_eq!(
        rows(&connection, &check, &OCTOBER_TO_NOVEMBER),
        [
            row(&[
                Some("2026-10-01"),
                Some("EAST"),
                None,
                Some("1"),
                None,
                Some("1.00"),
                Some("missing_in_base"),
            ]),
            row(&[
                Some("2026-10-01"),
                Some("NORTH"),
                Some("2"),
                Some("2"),
                Some("110.00"),
                Some("111.00"),
                Some("measure_differs"),
            ]),
            row(&[
                Some("2026-11-01"),
                Some("SOUTH"),
                Some("1"),
                None,
                Some("50.00"),
                None,
                Some("missing_in_summary"),
            ]),
        ]
    );
}

#[test]
fn chk_5_3_a_summary_passes_its_check_only_in_the_time_zone_it_was_built_in() {
    let document = document();
    let connection = database();
    let la_paz = ["2026-11-01", "2026-11-02", "America/La_Paz"];
    build_into(
        &connection,
        "shop.main.orders_daily_local (day_start, large_amount)",
        &sql(&document, DAILY, true),
        &la_paz,
    );
    let check = sql(&document, DAILY, false);
    assert!(rows(&connection, &check, &la_paz).is_empty());
    let utc = ["2026-11-01", "2026-11-02", "UTC"];
    assert_eq!(
        violations(&rows(&connection, &check, &utc)),
        ["missing_in_base", "missing_in_summary"]
    );
}

#[test]
fn chk_5_6_day_bounds_that_cover_whole_months_build_the_same_rows() {
    let document = document();
    let connection = database();
    let build = sql(&document, MONTHLY, true);
    assert_eq!(
        rows(&connection, &build, &["2026-10-01", "2026-11-30"]),
        rows(&connection, &build, &OCTOBER_TO_NOVEMBER)
    );
}

#[test]
fn chk_7_4_each_bound_is_read_from_its_shape() {
    let document = document();
    let connection = database();
    let build = sql(&document, MONTHLY, true);
    let fourth_quarter = rows(&connection, &build, &["2026-Q4", "2026-Q4"]);
    assert_eq!(fourth_quarter.len(), 4);
    assert_eq!(rows(&connection, &build, &["2026", "2026"]), fourth_quarter);
    // ISO week 41 of 2026 is 2026-10-05 to 2026-10-11.
    assert_eq!(
        rows(&connection, &build, &["2026-W41", "2026-W41"]),
        [row(&[
            Some("2026-10-01"),
            Some("1"),
            Some("NORTH"),
            Some("100.00")
        ])]
    );
}

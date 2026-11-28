//! Queries along temporal relationships through `core` (VAL-2): the `ClickHouse` SQL never
//! uses `ASOF JOIN` (VAL-2.23), the coverage condition adds no parameter and does not depend
//! on the query time zone (VAL-2.21), and an `open_end` outside the range of the `ClickHouse`
//! type of `valid_to` fails with `E6-01` in queries and in integrity checks (VAL-2.23).

#![cfg(test)]

use menura_core::{
    CompileOptions, CompileResult, Dialect, Handle, LoadOptions, compile, integrity_checks, load,
};

const OVERLAP_MODEL: &str =
    include_str!("../../../conformance/models/val-temporal-overlap/model.yaml");
const OVERLAP_SCHEMA: &str =
    include_str!("../../../conformance/models/val-temporal-overlap/catalog-schema.json");
const POLICY_MODEL: &str =
    include_str!("../../../conformance/models/val-temporal-policy/model.yaml");
const POLICY_SCHEMA: &str =
    include_str!("../../../conformance/models/val-temporal-policy/catalog-schema.json");
const FAR_END_MODEL: &str =
    include_str!("../../../conformance/models/val-temporal-far-end/model.yaml");
const FAR_END_SCHEMA: &str =
    include_str!("../../../conformance/models/val-temporal-far-end/catalog-schema.json");
const ANALYST: &str =
    include_str!("../../../conformance/models/val-temporal-overlap/contexts/analyst.json");

fn handle(model: &str, schema: &str) -> Handle {
    load(
        model.as_bytes(),
        schema.as_bytes(),
        None,
        &LoadOptions::default(),
    )
    .unwrap()
}

fn query(time_zone: &str, dimensions: &str, metric: &str) -> String {
    format!(
        r#"{{"contract": "menura.query", "contract_version": "1.1.0", "time_zone": "{time_zone}",
            "query": {{"dimensions": [{dimensions}],
              "metrics": [{{"metric": "{metric}", "alias": "{metric}"}}]}}}}"#
    )
}

fn compiled(handle: &Handle, query: &str, dialect: Dialect) -> CompileResult {
    let options = CompileOptions {
        dialect,
        ..CompileOptions::default()
    };
    compile(handle, query.as_bytes(), ANALYST.as_bytes(), &options).result
}

/// The SQL and the number of parameters of a compiled plan; fails the test otherwise.
fn sql(result: CompileResult) -> (String, usize) {
    match result {
        CompileResult::Plan(plan) => (plan.sql.clone(), plan.parameters.len()),
        CompileResult::Error(error) => panic!("expected a plan, got {error:?}"),
    }
}

#[test]
fn val_2_23_clickhouse_never_writes_an_asof_join() {
    let joined = sql(compiled(
        &handle(OVERLAP_MODEL, OVERLAP_SCHEMA),
        &query("UTC", r#""customer_tiers.tier""#, "visit_count"),
        Dialect::Clickhouse,
    ));
    let propagated = sql(compiled(
        &handle(POLICY_MODEL, POLICY_SCHEMA),
        &query("UTC", "", "sale_count"),
        Dialect::Clickhouse,
    ));
    for (text, _) in [joined, propagated] {
        assert!(!text.to_uppercase().contains("ASOF"), "{text}");
        assert!(text.contains("SETTINGS"), "{text}");
    }
}

#[test]
fn val_2_21_the_query_time_zone_changes_neither_the_sql_nor_the_parameters() {
    let handle = handle(OVERLAP_MODEL, OVERLAP_SCHEMA);
    for dialect in [Dialect::Duckdb, Dialect::Clickhouse] {
        let compile_in = |time_zone: &str| {
            sql(compiled(
                &handle,
                &query(time_zone, r#""customer_tiers.tier""#, "visit_count"),
                dialect,
            ))
        };
        let utc = compile_in("UTC");
        assert_eq!(utc.1, 0);
        assert_eq!(compile_in("Asia/Tokyo"), utc);
        assert_eq!(compile_in("America/Los_Angeles"), utc);
    }
}

#[test]
fn val_2_23_an_open_end_outside_the_clickhouse_range_is_e6_01_in_a_query() {
    let handle = handle(FAR_END_MODEL, FAR_END_SCHEMA);
    let query = query("UTC", r#""customer_versions.segment""#, "sale_count");
    let CompileResult::Error(error) = compiled(&handle, &query, Dialect::Clickhouse) else {
        panic!("expected E6-01");
    };
    let diagnostic = &error.errors[0];
    assert_eq!(diagnostic.code, "E6-01", "{diagnostic:?}");
    assert!(
        diagnostic.message.contains("dataset:CUSTOMER_VERSIONS"),
        "{diagnostic:?}"
    );
    let location = diagnostic.location.as_ref().unwrap();
    assert_eq!(location.pointer, "/semantic_model/0/datasets/1");
    sql(compiled(&handle, &query, Dialect::Duckdb));
}

#[test]
fn val_2_23_an_open_end_outside_the_clickhouse_range_is_e6_01_in_the_checks() {
    let handle = handle(FAR_END_MODEL, FAR_END_SCHEMA);
    let error = integrity_checks(&handle, Dialect::Clickhouse).unwrap_err();
    let diagnostic = &error.errors[0];
    assert_eq!(diagnostic.code, "E6-01", "{diagnostic:?}");
    assert!(
        diagnostic.message.contains("dataset:CUSTOMER_VERSIONS"),
        "{diagnostic:?}"
    );
    assert!(integrity_checks(&handle, Dialect::Duckdb).is_ok());
}

const VALIDITY_FAR_END_MODEL: &str =
    include_str!("../../../conformance/models/val-validity-far-end/model.yaml");
const VALIDITY_FAR_END_SCHEMA: &str =
    include_str!("../../../conformance/models/val-validity-far-end/catalog-schema.json");

#[test]
fn e6_7_12_the_open_end_of_a_population_outside_the_clickhouse_range_is_e6_01_in_the_checks() {
    let handle = handle(VALIDITY_FAR_END_MODEL, VALIDITY_FAR_END_SCHEMA);
    let error = integrity_checks(&handle, Dialect::Clickhouse).unwrap_err();
    let diagnostic = &error.errors[0];
    assert_eq!(diagnostic.code, "E6-01", "{diagnostic:?}");
    assert!(
        diagnostic.message.contains("dataset:MEMBERS"),
        "{diagnostic:?}"
    );
    assert!(integrity_checks(&handle, Dialect::Duckdb).is_ok());
}

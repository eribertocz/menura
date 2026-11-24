//! The `clickhouse` dialect through `core` (E6-7, E6-7.12, architecture section 3.5): the
//! compile option and the `integrity_checks` argument route to the `ClickHouse` generator, and
//! an unknown dialect is a usage error that lists both dialects (ARC-5.8).

#![cfg(test)]

mod support;

use menura_contracts::{ContractKind, validate_structure};
use menura_core::{
    CompileOptions, CompileResult, Dialect, Handle, IntegrityChecksDocument, LoadOptions, compile,
    explain, integrity_checks, integrity_checks_json, load, parse_compile_options, parse_dialect,
    plan_json,
};
use support::{amount_by_region, analyst, compile_default, error, handle, plan, query};

const CHECKS_MODEL: &str = include_str!("../../../conformance/models/chk-relationships/model.yaml");
const CHECKS_CATALOG_SCHEMA: &str =
    include_str!("../../../conformance/models/chk-relationships/catalog-schema.json");

fn clickhouse() -> CompileOptions {
    CompileOptions {
        dialect: Dialect::Clickhouse,
        ..CompileOptions::default()
    }
}

fn compile_clickhouse(handle: &Handle, query: &str, context: &str) -> CompileResult {
    compile(handle, query.as_bytes(), context.as_bytes(), &clickhouse()).result
}

fn checks_handle() -> Handle {
    load(
        CHECKS_MODEL.as_bytes(),
        CHECKS_CATALOG_SCHEMA.as_bytes(),
        None,
        &LoadOptions::default(),
    )
    .unwrap()
}

fn check_ids(document: &IntegrityChecksDocument) -> Vec<&str> {
    document
        .checks
        .iter()
        .map(|check| check.check_id.as_str())
        .collect()
}

/// A query of `orders.region` filtered by `CURRENT_TIMESTAMP`, a `timestamp` without time zone,
/// which `ClickHouse` cannot express (E6-7.12).
fn region_with_current_timestamp() -> String {
    query(
        r#"{"dimensions": ["orders.region"], "nonportable_extensions": [{"vendor_name": "COMMON",
            "data": {"namespace": "menura", "version": 1,
              "filters": ["CURRENT_TIMESTAMP IS NOT NULL"]}}]}"#,
    )
    .replace(
        r#""time_zone": "UTC""#,
        r#""reference_time": "2026-02-20T12:00:00Z", "time_zone": "UTC""#,
    )
}

#[test]
fn e6_7_compile_with_clickhouse_returns_a_clickhouse_plan() {
    let compiled = plan(compile_clickhouse(
        &handle(),
        &amount_by_region(),
        &analyst(),
    ));
    assert_eq!(compiled.dialect, Dialect::Clickhouse);
    assert!(compiled.sql.contains("SETTINGS"), "{}", compiled.sql);
    assert!(compiled.sql.contains("{p1:"), "{}", compiled.sql);
    let duckdb = plan(compile_default(&handle(), &amount_by_region(), &analyst()));
    assert_eq!(duckdb.dialect, Dialect::Duckdb);
    assert_ne!(compiled.sql, duckdb.sql);
    assert_ne!(compiled.cache_key.plan_hash, duckdb.cache_key.plan_hash);
    assert_eq!(compiled.lineage, duckdb.lineage);
}

#[test]
fn e6_7_the_clickhouse_plan_validates_against_its_schema() {
    let compiled = plan(compile_clickhouse(
        &handle(),
        &amount_by_region(),
        &analyst(),
    ));
    let json: serde_json::Value = serde_json::from_str(&plan_json(&compiled).unwrap()).unwrap();
    validate_structure(ContractKind::CompiledPlan, &json).unwrap();
    assert_eq!(json["dialect"], "clickhouse");
}

#[test]
fn e6_7_explain_with_clickhouse_is_the_clickhouse_plan_without_sql() {
    let handle = handle();
    let compiled = plan(compile_clickhouse(&handle, &amount_by_region(), &analyst()));
    let explained = plan(explain(
        &handle,
        amount_by_region().as_bytes(),
        analyst().as_bytes(),
        &clickhouse(),
    ));
    assert_eq!(explained.dialect, Dialect::Clickhouse);
    assert!(explained.sql.is_empty());
    assert!(explained.parameters.is_empty());
    assert_eq!(explained.cache_key, compiled.cache_key);
}

#[test]
fn e6_7_12_e6_01_comes_back_through_core_as_a_compile_error() {
    let handle = handle();
    let query = region_with_current_timestamp();
    let error = error(compile_clickhouse(&handle, &query, &analyst()));
    assert_eq!(error.errors.len(), 1);
    let diagnostic = &error.errors[0];
    assert_eq!(diagnostic.code, "E6-01");
    assert!(diagnostic.location.is_some());
    plan(compile_default(&handle, &query, &analyst()));
}

#[test]
fn e6_7_12_explain_returns_e6_01_as_compile_does() {
    let handle = handle();
    let query = region_with_current_timestamp();
    let explained = error(explain(
        &handle,
        query.as_bytes(),
        analyst().as_bytes(),
        &clickhouse(),
    ));
    assert_eq!(
        explained,
        error(compile_clickhouse(&handle, &query, &analyst()))
    );
}

#[test]
fn e6_7_11_integrity_checks_with_clickhouse_are_generated_for_clickhouse() {
    let handle = checks_handle();
    let clickhouse = integrity_checks(&handle, Dialect::Clickhouse).unwrap();
    let duckdb = integrity_checks(&handle, Dialect::Duckdb).unwrap();
    assert_eq!(clickhouse.dialect, Dialect::Clickhouse);
    assert!(!clickhouse.checks.is_empty());
    assert_eq!(check_ids(&clickhouse), check_ids(&duckdb));
    for (generated, reference) in clickhouse.checks.iter().zip(&duckdb.checks) {
        assert_eq!(generated.sources, reference.sources);
        assert_eq!(generated.definition_hash, reference.definition_hash);
        assert!(generated.sql.contains("SETTINGS"), "{}", generated.sql);
    }
}

#[test]
fn e6_7_11_the_clickhouse_checks_validate_against_their_schema() {
    let document = integrity_checks(&checks_handle(), Dialect::Clickhouse).unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&integrity_checks_json(&document).unwrap()).unwrap();
    validate_structure(ContractKind::IntegrityChecks, &json).unwrap();
    assert_eq!(json["dialect"], "clickhouse");
}

#[test]
fn arc_5_8_the_dialect_argument_accepts_clickhouse_and_duckdb() {
    assert_eq!(parse_dialect("clickhouse"), Ok(Dialect::Clickhouse));
    assert_eq!(parse_dialect("duckdb"), Ok(Dialect::Duckdb));
}

#[test]
fn arc_5_8_the_usage_error_of_an_unknown_dialect_lists_both_dialects() {
    let error = parse_dialect("postgres").unwrap_err();
    assert_eq!(
        error.to_string(),
        "unknown dialect `postgres`; the dialects are `clickhouse` and `duckdb`"
    );
}

#[test]
fn arc_5_7_the_compile_option_dialect_accepts_clickhouse() {
    let options = parse_compile_options(br#"{"dialect": "clickhouse"}"#).unwrap();
    assert_eq!(options.dialect, Dialect::Clickhouse);
    let error = parse_compile_options(br#"{"dialect": "postgres"}"#).unwrap_err();
    assert!(
        error
            .to_string()
            .ends_with("the compile option `dialect` must be `clickhouse` or `duckdb`"),
        "{error}"
    );
}

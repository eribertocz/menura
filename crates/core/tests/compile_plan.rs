//! The compiled plan of `compile`: PLN-1, PLN-2.4, E6-5.5, E6-6.1 and
//! the compile options of architecture section 3.5.

mod support;

use std::num::NonZeroU64;

use menura_contracts::types::compiled_plan::CompiledPlanContract;
use menura_contracts::{ContractKind, validate_structure};
use menura_core::{CompileOptions, Dialect, LoadOptions, compile, plan_json};
use support::{
    CATALOG_SCHEMA, MODEL, amount_by_region, analyst, compile_default, handle, handle_with, plan,
};

/// The commit `conformance/ossie/upstream.json` tracks.
const TRACKED_COMMIT: &str = "aa500d37894c3d62c96cdce4192cbecea4d7f06a";

fn compiled() -> menura_core::CompiledPlanDocument {
    plan(compile_default(&handle(), &amount_by_region(), &analyst()))
}

#[test]
fn pln_1_the_plan_validates_against_its_schema() {
    let json: serde_json::Value = serde_json::from_str(&plan_json(&compiled()).unwrap()).unwrap();
    validate_structure(ContractKind::CompiledPlan, &json).unwrap();
    assert_eq!(json["contract"], "menura.compiled_plan");
    assert_eq!(
        json["contract_version"],
        ContractKind::CompiledPlan.latest_version().to_string()
    );
}

#[test]
fn pln_1_the_plan_is_a_compiled_plan() {
    assert_eq!(compiled().contract, CompiledPlanContract::CompiledPlan);
}

#[test]
fn pln_1_producer_names_the_compiler_ossie_and_tzdata_versions() {
    let producer = compiled().producer;
    assert_eq!(
        producer.compiler_version.to_string(),
        env!("CARGO_PKG_VERSION")
    );
    assert_eq!(producer.ossie_normative, "0.1.1");
    assert_eq!(producer.ossie_tracked_commit, TRACKED_COMMIT);
}

#[test]
fn e6_5_5_producer_declares_the_time_zone_database_version() {
    let producer = compiled().producer;
    assert_eq!(
        producer.tzdata_version,
        menura_e1_parse::time_zone_database_version()
    );
    assert!(!producer.tzdata_version.is_empty());
}

#[test]
fn pln_1_dialect_is_duckdb() {
    let json: serde_json::Value = serde_json::from_str(&plan_json(&compiled()).unwrap()).unwrap();
    assert_eq!(json["dialect"], "duckdb");
}

#[test]
fn pln_1_data_source_defaults_to_the_catalog_schema_origin() {
    assert_eq!(compiled().data_source, "warehouse:shop");
}

#[test]
fn pln_1_data_source_comes_from_the_load_option() {
    let options = LoadOptions {
        data_source: Some("warehouse:gold".to_owned()),
        ..LoadOptions::default()
    };
    let handle = handle_with(MODEL, CATALOG_SCHEMA, None, &options);
    let compiled = plan(compile_default(&handle, &amount_by_region(), &analyst()));
    assert_eq!(compiled.data_source, "warehouse:gold");
}

#[test]
fn e6_6_1_sql_parameters_and_result_schema_come_from_e6() {
    let compiled = compiled();
    assert!(compiled.sql.contains("\"shop\".\"orders\""));
    assert_eq!(compiled.parameters.len(), 1);
    assert_eq!(
        compiled.parameters[0].value,
        serde_json::json!(["NORTH", "SOUTH"])
    );
    let names: Vec<&str> = compiled
        .result_schema
        .iter()
        .map(|column| column.name.as_str())
        .collect();
    assert_eq!(names, ["region", "total_amount"]);
}

#[test]
fn pln_1_limits_default_to_30000_ms_and_no_row_limit() {
    let limits = compiled().limits;
    assert_eq!(limits.timeout_ms, 30000);
    assert_eq!(limits.max_rows, None);
}

#[test]
fn pln_1_limits_come_from_the_compile_options() {
    let options = CompileOptions {
        timeout_ms: NonZeroU64::new(5000).unwrap(),
        max_rows: NonZeroU64::new(100),
        ..CompileOptions::default()
    };
    let result = compile(
        &handle(),
        amount_by_region().as_bytes(),
        analyst().as_bytes(),
        &options,
    )
    .result;
    let limits = plan(result).limits;
    assert_eq!(limits.timeout_ms, 5000);
    assert_eq!(limits.max_rows, Some(100));
}

#[test]
fn pln_1_the_default_dialect_option_is_duckdb() {
    assert_eq!(CompileOptions::default().dialect, Dialect::Duckdb);
}

#[test]
fn pln_1_a_plan_without_former_names_has_no_warnings() {
    assert!(compiled().warnings.is_empty());
}

#[test]
fn pln_2_4_compiling_twice_gives_byte_identical_output() {
    let first = plan_json(&compiled()).unwrap();
    let second = plan_json(&compiled()).unwrap();
    assert_eq!(first, second);
}

#[test]
fn pln_2_4_the_output_does_not_depend_on_the_key_order_of_the_inputs() {
    let reordered_query = r#"{"query": {"metrics": ["total_amount"], "dimensions": ["orders.region"]},
        "time_zone": "UTC", "contract_version": "1.1.0", "contract": "menura.query"}"#;
    let reordered_context = r#"{"attributes": {"regions": ["NORTH", "SOUTH"]}, "roles": ["analyst"],
        "subject": "analyst-1", "contract_version": "1.1.0", "contract": "menura.security_context"}"#;
    let reordered = plan(compile_default(
        &handle(),
        reordered_query,
        reordered_context,
    ));
    assert_eq!(
        plan_json(&reordered).unwrap(),
        plan_json(&compiled()).unwrap()
    );
}

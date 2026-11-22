//! Runs a prepared Apache Ossie case through `core` (CS-4.4, CS-4.6): the model is loaded over
//! the catalog schema built from the created tables (CS-4.4), the query envelope is compiled for the runner
//! security context (CS-4.5), and the plan's rows on the fixture are compared with the expected
//! rows, or the first error code with the expected one.

use duckdb::Connection;
use menura_contracts::types::audit_record;
use menura_core::{
    CompileOptions, CompileResult, CompiledPlanDocument, LoadOptions, compile, load,
};
use serde_json::Value;

use crate::diagnostic_check::describe;
use crate::execution::{ExecutionFailure, execute_plan};
use crate::ossie::catalog_schema::catalog_schema_document;
use crate::ossie::comparison::rows_mismatch;
use crate::ossie::expected::{ExpectedResult, ExpectedRows};
use crate::ossie::outer_order::has_outer_order_by;
use crate::ossie::prepare::PreparedCase;

/// Why the case `case_key`, prepared as `prepared` on `connection`, does not pass, or `None`
/// when it passes.
pub(crate) fn query_mismatch(
    case_key: &str,
    prepared: &PreparedCase,
    connection: &Connection,
) -> Option<String> {
    let engine_version = match engine_version(connection) {
        Ok(version) => version,
        Err(error) => return Some(format!("the DuckDB version cannot be read: {error}")),
    };
    let catalog = catalog_schema_document(case_key, &engine_version, &prepared.created_tables);
    let handle = match load(
        prepared.model_text.as_bytes(),
        catalog.to_string().as_bytes(),
        None,
        &core_load_options(&prepared.load_options),
    ) {
        Ok(handle) => handle,
        Err(error) => {
            return Some(format!(
                "the model does not load: {}",
                describe(error.errors.first())
            ));
        }
    };
    let result = compile(
        &handle,
        prepared.query.to_string().as_bytes(),
        prepared.security_context.to_string().as_bytes(),
        &CompileOptions::default(),
    )
    .result;
    match (&prepared.expected, result) {
        (ExpectedResult::Error(code), CompileResult::Error(error)) => {
            let first = error.errors.first();
            (first.map(|diagnostic| diagnostic.code.as_str()) != Some(code.as_str()))
                .then(|| format!("the first error is {}, expected {code}", describe(first)))
        }
        (ExpectedResult::Error(code), CompileResult::Plan(_)) => Some(format!(
            "the query compiles, but the case expects the error {code}"
        )),
        (ExpectedResult::Rows(_), CompileResult::Error(error)) => Some(format!(
            "the query does not compile: {}",
            describe(error.errors.first())
        )),
        (ExpectedResult::Rows(expected), CompileResult::Plan(plan)) => {
            plan_rows_mismatch(&plan, expected, ordered(&prepared.query), connection)
        }
    }
}

/// Executes `plan` on `connection` and compares its rows with `expected`.
fn plan_rows_mismatch(
    plan: &CompiledPlanDocument,
    expected: &ExpectedRows,
    ordered: bool,
    connection: &Connection,
) -> Option<String> {
    match execute_plan(connection, plan) {
        Ok(rows) => rows_mismatch(&plan.result_schema, &rows, expected, ordered),
        Err(ExecutionFailure::Parameter(reason)) => {
            Some(format!("the plan cannot be bound: {reason}"))
        }
        Err(ExecutionFailure::Engine(message)) => {
            Some(format!("the plan fails on DuckDB: {message}"))
        }
    }
}

/// Whether the SQL text of the query envelope has an outer `ORDER BY`.
fn ordered(query: &Value) -> bool {
    query
        .get("sql")
        .and_then(Value::as_str)
        .is_some_and(has_outer_order_by)
}

fn engine_version(connection: &Connection) -> Result<String, duckdb::Error> {
    connection.query_row("SELECT version()", [], |row| row.get(0))
}

/// The load options of CS-4.4 as the `load` call reads them.
fn core_load_options(options: &audit_record::LoadOptions) -> LoadOptions {
    LoadOptions {
        model_name: options.model_name.clone(),
        ossie_format: options.ossie_format,
        grant_authenticated_to_all: options.grant_authenticated_to_all,
        namespace: options.namespace.clone(),
        data_source: None,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn cs_4_6_the_row_order_is_compared_only_for_an_ordered_sql_query() {
        assert!(ordered(&json!({"sql": "SELECT a FROM m.d ORDER BY a"})));
        assert!(!ordered(&json!({"sql": "SELECT a FROM m.d"})));
        assert!(!ordered(&json!({"query": {"dimensions": ["d.a"]}})));
    }
}

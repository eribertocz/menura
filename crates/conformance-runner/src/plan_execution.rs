//! The execution part of a case (E6-5.4): a compiled plan whose golden files match is executed
//! on the case's database. A `runtime_error` case must fail with its code (CS-2.5), and any
//! other case must return its rows (CS-2.4) and, when differential, those of its oracle
//! (CS-4.7).

use std::path::Path;

use duckdb::Connection;
use menura_core::CompiledPlanDocument;

use crate::case::{Case, Expectation, Outcome};
use crate::csv_rows::parse_csv;
use crate::differential::check_differential;
use crate::divergences::Divergences;
use crate::execution::{ExecutionFailure, execute_plan};
use crate::fixture::read_text;
use crate::report::CaseStatus;
use crate::row_comparison::{Comparison, rows_mismatch};

/// What a plan is executed with: the case's database and the reviewed divergences.
pub(crate) struct CaseDatabase<'database> {
    /// The prepared database of the case (CS-4.2).
    pub(crate) connection: &'database Connection,
    /// The classified divergences of differential cases (CS-4.8).
    pub(crate) divergences: &'database Divergences,
}

/// Executes `plan` on the database of `case` and compares the result with its expectation; the
/// `rows` and `oracle` files of the case are in `case_folder`.
pub(crate) fn check_execution(
    case: &Case,
    plan: &CompiledPlanDocument,
    case_folder: &Path,
    database: &CaseDatabase<'_>,
) -> CaseStatus {
    if case.expect.outcome == Outcome::RuntimeError {
        return check_runtime_error(&case.expect, plan, database.connection);
    }
    match check_rows(&case.expect, plan, case_folder, database.connection) {
        CaseStatus::Passed if case.oracle.is_some() => {
            check_differential(case, plan, case_folder, database)
        }
        status => status,
    }
}

/// Executes the plan and compares the rows it returns on `DuckDB` with the expected rows
/// (CS-2.4, E6-5.4).
fn check_rows(
    expect: &Expectation,
    plan: &CompiledPlanDocument,
    case_folder: &Path,
    connection: &Connection,
) -> CaseStatus {
    match compare_rows(expect, plan, case_folder, connection) {
        Ok(None) => CaseStatus::Passed,
        Ok(Some(reason)) | Err(reason) => CaseStatus::Failed(reason),
    }
}

/// Executes the plan and compares its rows with the `rows` file of the case.
fn compare_rows(
    expect: &Expectation,
    plan: &CompiledPlanDocument,
    case_folder: &Path,
    connection: &Connection,
) -> Result<Option<String>, String> {
    let file_name = expect
        .rows
        .as_deref()
        .ok_or_else(|| "`outcome: plan` requires `rows`".to_owned())?;
    let text = read_text(&case_folder.join(file_name)).map_err(|error| error.to_string())?;
    let expected =
        parse_csv(&text).map_err(|reason| format!("the rows file `{file_name}`: {reason}"))?;
    let actual = execute_plan(connection, plan).map_err(|failure| match failure {
        ExecutionFailure::Parameter(reason) => format!("the plan cannot be bound: {reason}"),
        ExecutionFailure::Engine(message) => format!("the plan fails on DuckDB: {message}"),
    })?;
    Ok(rows_mismatch(
        &plan.result_schema,
        &actual,
        &expected,
        Comparison::of(expect),
    ))
}

/// Executes the plan of a `runtime_error` case, which must fail with the expected runtime code
/// in the engine's message (CS-2.5).
fn check_runtime_error(
    expect: &Expectation,
    plan: &CompiledPlanDocument,
    connection: &Connection,
) -> CaseStatus {
    let code = expect.error.as_deref().unwrap_or("none");
    match execute_plan(connection, plan) {
        Err(ExecutionFailure::Engine(message)) if message.contains(code) => CaseStatus::Passed,
        Err(ExecutionFailure::Engine(message)) => CaseStatus::Failed(format!(
            "the plan fails on DuckDB without the runtime code {code}: {message}"
        )),
        Err(ExecutionFailure::Parameter(reason)) => {
            CaseStatus::Failed(format!("the plan cannot be bound: {reason}"))
        }
        Ok(_) => CaseStatus::Failed(format!(
            "the plan executes, but the case expects the runtime code {code}"
        )),
    }
}

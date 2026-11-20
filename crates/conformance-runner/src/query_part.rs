//! The query part of a case (CS-2.8): the query is compiled through `core` for the case's
//! security context, and the result is compared with the expectation. A compilation that returns
//! `E_NOT_YET_SUPPORTED` leaves the part pending (VS-4), unless the case itself expects that
//! error, which is then compared like any other.

use std::path::Path;

use duckdb::Connection;
use menura_core::{
    CompileErrorDocument, CompileOptions, CompileResult, CompiledPlanDocument, Diagnostic, Handle,
    compile,
};

use crate::case::{Case, Expectation, Outcome, RowOrder};
use crate::csv_rows::parse_csv;
use crate::diagnostic_check::{Part, describe, location_mismatch, missing_warnings};
use crate::execution::{ExecutionFailure, execute_plan};
use crate::fixture::read_text;
use crate::report::CaseStatus;
use crate::row_comparison::{Comparison, rows_mismatch};

/// The code that leaves a query part pending (VS-4).
const NOT_YET_SUPPORTED: &str = "E_NOT_YET_SUPPORTED";

/// What the query part of a case reads besides the case and the loaded model.
pub(crate) struct QueryInputs<'inputs> {
    /// The folder of the case file, where its `sql` and `rows` files are.
    pub(crate) case_folder: &'inputs Path,
    /// The security context document, as bytes.
    pub(crate) security_context: &'inputs [u8],
    /// The prepared database of the case (CS-4.2).
    pub(crate) connection: &'inputs Connection,
}

/// Compiles the query of `case` over `handle` and compares the result with its expectation.
pub(crate) fn check_query(case: &Case, handle: &Handle, inputs: &QueryInputs<'_>) -> CaseStatus {
    let Some(query) = &case.query else {
        return CaseStatus::Failed("this outcome requires `query`".to_owned());
    };
    let query = match serde_json::to_vec(query) {
        Ok(query) => query,
        Err(error) => return CaseStatus::Failed(format!("the query is not JSON: {error}")),
    };
    let result = compile_query(handle, &query, inputs.security_context);
    if let Some(reason) = missing_warnings(&case.expect.warnings, warnings_of(&result), Part::Query)
    {
        return CaseStatus::Failed(reason);
    }
    match (case.expect.outcome, result) {
        (_, CompileResult::Error(error))
            if first_code(&error) == Some(NOT_YET_SUPPORTED)
                && !expects_not_yet_supported(&case.expect) =>
        {
            CaseStatus::Pending(format!(
                "the query part needs a capability outside this version's slice (VS-4): {}",
                describe(error.errors.first())
            ))
        }
        (Outcome::Error, CompileResult::Error(error)) => check_error(&case.expect, &error),
        (Outcome::Error, CompileResult::Plan(_)) => CaseStatus::Failed(format!(
            "the query compiles, but the case expects the error {}",
            case.expect.error.as_deref().unwrap_or("none")
        )),
        (Outcome::LoadError, _) => {
            CaseStatus::Failed("a `load_error` case has no query part".to_owned())
        }
        (_, CompileResult::Error(error)) => CaseStatus::Failed(format!(
            "the query does not compile: {}",
            describe(error.errors.first())
        )),
        (Outcome::Plan, CompileResult::Plan(plan)) => check_plan(case, &plan, inputs, || {
            compile_query(handle, &query, inputs.security_context)
        }),
        (Outcome::RuntimeError, CompileResult::Plan(plan)) => {
            check_runtime_error(&case.expect, &plan, inputs.connection)
        }
    }
}

/// Whether the case itself expects `E_NOT_YET_SUPPORTED`, so that the error is compared, not
/// left pending (VS-4).
fn expects_not_yet_supported(expect: &Expectation) -> bool {
    expect.outcome == Outcome::Error && expect.error.as_deref() == Some(NOT_YET_SUPPORTED)
}

fn compile_query(handle: &Handle, query: &[u8], security_context: &[u8]) -> CompileResult {
    compile(handle, query, security_context, &CompileOptions::default())
}

fn warnings_of(result: &CompileResult) -> std::slice::Iter<'_, Diagnostic> {
    match result {
        CompileResult::Plan(plan) => plan.warnings.iter(),
        CompileResult::Error(error) => error.warnings.iter(),
    }
}

fn first_code(error: &CompileErrorDocument) -> Option<&str> {
    error
        .errors
        .first()
        .map(|diagnostic| diagnostic.code.as_str())
}

/// Compares a compile error with the expectation of an `error` case: the first code, and its
/// location when the case gives one (CS-2.9).
fn check_error(expect: &Expectation, error: &CompileErrorDocument) -> CaseStatus {
    let first = error.errors.first();
    if first_code(error) != expect.error.as_deref() {
        return CaseStatus::Failed(format!(
            "the first error is {}, expected {}",
            describe(first),
            expect.error.as_deref().unwrap_or("none")
        ));
    }
    let reported = first.and_then(|diagnostic| diagnostic.location.as_ref());
    match expect
        .location
        .as_ref()
        .and_then(|expected| location_mismatch(expected, reported, Part::Query))
    {
        Some(reason) => CaseStatus::Failed(reason),
        None => CaseStatus::Passed,
    }
}

/// Checks a compiled plan: a second compilation gives the same plan (P3, PLN-3), the SQL matches
/// its golden file when the case gives one (CS-2.3), and the rows it returns on `DuckDB` match
/// the expected rows (CS-2.4, E6-5.4).
fn check_plan(
    case: &Case,
    plan: &CompiledPlanDocument,
    inputs: &QueryInputs<'_>,
    compile_again: impl FnOnce() -> CompileResult,
) -> CaseStatus {
    if !matches!(compile_again(), CompileResult::Plan(again) if *again == *plan) {
        return CaseStatus::Failed(
            "two compilations of the same inputs give different results".to_owned(),
        );
    }
    if let Some(file_name) = &case.expect.sql {
        match read_text(&inputs.case_folder.join(file_name)) {
            Ok(golden) if golden == plan.sql => {}
            Ok(_) => {
                return CaseStatus::Failed(format!(
                    "the SQL differs from the golden file `{file_name}`; the SQL is:\n{}",
                    plan.sql
                ));
            }
            Err(error) => return CaseStatus::Failed(error.to_string()),
        }
    }
    match compare_rows(&case.expect, plan, inputs) {
        Ok(None) => CaseStatus::Passed,
        Ok(Some(reason)) | Err(reason) => CaseStatus::Failed(reason),
    }
}

/// Executes the plan and compares its rows with the `rows` file of the case.
fn compare_rows(
    expect: &Expectation,
    plan: &CompiledPlanDocument,
    inputs: &QueryInputs<'_>,
) -> Result<Option<String>, String> {
    let file_name = expect
        .rows
        .as_deref()
        .ok_or_else(|| "`outcome: plan` requires `rows`".to_owned())?;
    let text = read_text(&inputs.case_folder.join(file_name)).map_err(|error| error.to_string())?;
    let expected =
        parse_csv(&text).map_err(|reason| format!("the rows file `{file_name}`: {reason}"))?;
    let actual = execute_plan(inputs.connection, plan).map_err(|failure| match failure {
        ExecutionFailure::Parameter(reason) => format!("the plan cannot be bound: {reason}"),
        ExecutionFailure::Engine(message) => format!("the plan fails on DuckDB: {message}"),
    })?;
    let comparison = Comparison {
        row_order: expect.row_order.unwrap_or(RowOrder::Any),
        tolerance: expect.tolerance,
    };
    Ok(rows_mismatch(
        &plan.result_schema,
        &actual,
        &expected,
        comparison,
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::case_file::parse_case;
    use crate::database::prepare_database;
    use crate::fixture::{ModelFixture, read_bytes, security_context_path};
    use crate::load_options::resolve_load_options;
    use crate::load_part::{LoadOutcome, ModelDocuments, check_load};

    fn conformance_directory() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
    }

    /// Runs the query part of a case of the `slice` model, whose files are under `cases/cs/`.
    fn status(case_text: &str) -> CaseStatus {
        let case = parse_case(case_text).unwrap();
        let model_folder = conformance_directory().join("models/slice");
        let connection = prepare_database(&ModelFixture::read(&model_folder).unwrap()).unwrap();
        let documents = ModelDocuments::read(&model_folder).unwrap();
        let LoadOutcome::Loaded(handle) =
            check_load(&case, &documents, &resolve_load_options(None))
        else {
            panic!("the slice model loads");
        };
        let context_name = case.security_context.as_deref().unwrap();
        let security_context =
            read_bytes(&security_context_path(&model_folder, context_name)).unwrap();
        let case_folder = conformance_directory().join("cases/cs");
        let inputs = QueryInputs {
            case_folder: &case_folder,
            security_context: &security_context,
            connection: &connection,
        };
        check_query(&case, &handle, &inputs)
    }

    fn case_text(query: &str, expect: &str) -> String {
        format!(
            "id: cs-unit-001\nrules: [CS-2.8]\nmodel: slice\nsecurity_context: auditor.json\n\
             query: {{contract: menura.query, contract_version: \"1.1.0\", time_zone: UTC, \
             query: {query}}}\nexpect: {expect}\n"
        )
    }

    const BY_REGION: &str = "{dimensions: [sales.region], metrics: [total_amount, total_quantity, \
                             sale_count]}";

    fn assert_fails(status: CaseStatus, words: &str) {
        let CaseStatus::Failed(reason) = status else {
            panic!("the case fails: {status:?}");
        };
        assert!(reason.contains(words), "{reason}");
    }

    #[test]
    fn cs_2_8_a_plan_case_passes_when_its_rows_match() {
        let expect = "{outcome: plan, rows: cs-plan-rows-001.csv}";
        assert_eq!(status(&case_text(BY_REGION, expect)), CaseStatus::Passed);
    }

    #[test]
    fn vs_4_a_query_outside_the_slice_is_pending_whatever_the_expectation() {
        let query = "{metrics: [total_amount, total_target]}";
        let status = status(&case_text(
            query,
            "{outcome: plan, rows: cs-plan-rows-001.csv}",
        ));
        let CaseStatus::Pending(reason) = status else {
            panic!("the case is pending: {status:?}");
        };
        assert!(reason.contains(NOT_YET_SUPPORTED), "{reason}");
    }

    #[test]
    fn vs_4_a_case_that_expects_not_yet_supported_compares_it() {
        let query = "{metrics: [total_amount, total_target]}";
        let expect = |pointer: &str| {
            format!(
                "{{outcome: error, error: E_NOT_YET_SUPPORTED, location: {{document: query, \
                 pointer: {pointer}}}}}"
            )
        };
        assert_eq!(
            status(&case_text(query, &expect("/query/metrics/1"))),
            CaseStatus::Passed
        );
        assert_fails(
            status(&case_text(query, &expect("/query/metrics/0"))),
            "location",
        );
    }

    #[test]
    fn cs_2_8_an_error_case_fails_when_the_query_compiles() {
        let expect = "{outcome: error, error: E_NAME_NOT_FOUND}";
        assert_fails(status(&case_text(BY_REGION, expect)), "compiles");
    }

    #[test]
    fn cs_2_8_a_plan_case_fails_when_the_query_does_not_compile() {
        let expect = "{outcome: plan, rows: cs-plan-rows-001.csv}";
        let query = "{dimensions: [sales.nowhere]}";
        assert_fails(status(&case_text(query, expect)), "does not compile");
    }

    #[test]
    fn cs_2_9_an_error_case_fails_with_another_location() {
        let expect = "{outcome: error, error: E_NAME_NOT_FOUND, location: {document: query, \
                      pointer: /query/metrics/0}}";
        let query = "{dimensions: [sales.nowhere]}";
        assert_fails(status(&case_text(query, expect)), "location");
    }

    #[test]
    fn cs_2_3_a_plan_case_fails_when_its_sql_differs_from_the_golden_file() {
        let expect = "{outcome: plan, sql: cs-plan-rows-001.csv, rows: cs-plan-rows-001.csv}";
        assert_fails(status(&case_text(BY_REGION, expect)), "golden");
    }

    #[test]
    fn cs_2_4_a_plan_case_fails_when_its_rows_differ() {
        let expect = "{outcome: plan, rows: e1-output-name-001.csv}";
        let status = status(&case_text(BY_REGION, expect));
        assert!(matches!(status, CaseStatus::Failed(_)), "{status:?}");
    }

    #[test]
    fn cs_2_5_a_runtime_error_case_fails_when_the_plan_executes() {
        let expect = "{outcome: runtime_error, error: RADD-01}";
        assert_fails(status(&case_text(BY_REGION, expect)), "executes");
    }

    #[test]
    fn cs_2_8_an_expected_warning_must_be_reported_by_the_compilation() {
        let expect = "{outcome: plan, rows: cs-plan-rows-001.csv, warnings: [WQRY-01]}";
        assert_fails(status(&case_text(BY_REGION, expect)), "WQRY-01");
    }
}

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

use crate::case::{Case, Expectation, Outcome};
use crate::csv_rows::parse_csv;
use crate::diagnostic_check::{Part, describe, location_mismatch, missing_warnings};
use crate::differential::check_differential;
use crate::divergences::Divergences;
use crate::execution::{ExecutionFailure, execute_plan};
use crate::explain_part::{explained_mismatch, runs_explain};
use crate::fixture::read_text;
use crate::pending_parts::with_pending_parts;
use crate::plan_outputs::plan_outputs_mismatch;
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
    /// The classified divergences of differential cases (CS-4.8).
    pub(crate) divergences: &'inputs Divergences,
}

/// Compiles the query of `case` over `handle` and compares the result with its expectation. A
/// part that needs a `core` function this build lacks leaves the case pending (CS-2.8).
pub(crate) fn check_query(case: &Case, handle: &Handle, inputs: &QueryInputs<'_>) -> CaseStatus {
    with_pending_parts(case, compare_query(case, handle, inputs))
}

/// Compiles the query of `case`, with `audit: true` when the case expects an audit record
/// (CS-2.13), and compares the result with its expectation. A passed `outcome: plan` case also
/// runs `explain` from 0.7.0, whose document must match the plan (CS-2.16).
fn compare_query(case: &Case, handle: &Handle, inputs: &QueryInputs<'_>) -> CaseStatus {
    let Some(query) = &case.query else {
        return CaseStatus::Failed("this outcome requires `query`".to_owned());
    };
    let query = match serde_json::to_vec(query) {
        Ok(query) => query,
        Err(error) => return CaseStatus::Failed(format!("the query is not JSON: {error}")),
    };
    let options = compile_options(case);
    let compile_query = || compile(handle, &query, inputs.security_context, &options);
    let result = compile_query();
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
        (_, CompileResult::Plan(plan)) => {
            let explain_query = || explain(handle, &query, inputs.security_context, &options);
            match check_plan(case, &plan, inputs, compile_query) {
                CaseStatus::Passed if runs_explain(case) => {
                    explained_mismatch(&plan, &explain_query())
                        .map_or(CaseStatus::Passed, CaseStatus::Failed)
                }
                status => status,
            }
        }
    }
}

/// Checks a compiled plan before execution, then executes it: a `runtime_error` case must fail
/// with its code, and any other case must return its rows and, when differential, those of its
/// oracle.
fn check_plan(
    case: &Case,
    plan: &CompiledPlanDocument,
    inputs: &QueryInputs<'_>,
    compile_again: impl FnOnce() -> CompileResult,
) -> CaseStatus {
    if let Some(reason) = plan_mismatch(case, plan, inputs.case_folder, compile_again) {
        return CaseStatus::Failed(reason);
    }
    if case.expect.outcome == Outcome::RuntimeError {
        return check_runtime_error(&case.expect, plan, inputs.connection);
    }
    match check_rows(&case.expect, plan, inputs) {
        CaseStatus::Passed if case.oracle.is_some() => check_differential(case, plan, inputs),
        status => status,
    }
}

/// The compile options of a case: `audit: true` only when it expects an audit record
/// (CS-2.13).
fn compile_options(case: &Case) -> CompileOptions {
    CompileOptions {
        audit: case.expect.audit.is_some(),
        ..CompileOptions::default()
    }
}

/// Whether the case itself expects `E_NOT_YET_SUPPORTED`, so that the error is compared, not
/// left pending (VS-4).
fn expects_not_yet_supported(expect: &Expectation) -> bool {
    expect.outcome == Outcome::Error && expect.error.as_deref() == Some(NOT_YET_SUPPORTED)
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

/// Why a compiled plan does not meet the checks made before it is executed, or `None`: a second
/// compilation gives the same plan (P3, PLN-3), the SQL matches its golden file when the case
/// gives one (CS-2.3), and the plan meets its output expectations (CS-2.11, CS-2.12, CS-2.14).
fn plan_mismatch(
    case: &Case,
    plan: &CompiledPlanDocument,
    case_folder: &Path,
    compile_again: impl FnOnce() -> CompileResult,
) -> Option<String> {
    if !matches!(compile_again(), CompileResult::Plan(again) if *again == *plan) {
        return Some("two compilations of the same inputs give different results".to_owned());
    }
    let Some(file_name) = case.expect.sql.as_ref() else {
        return plan_outputs_mismatch(&case.expect, plan, case_folder);
    };
    match read_text(&case_folder.join(file_name)) {
        Ok(golden) if golden == plan.sql => plan_outputs_mismatch(&case.expect, plan, case_folder),
        Ok(_) => Some(format!(
            "the SQL differs from the golden file `{file_name}`; the SQL is:\n{}",
            plan.sql
        )),
        Err(error) => Some(error.to_string()),
    }
}

/// Executes the plan and compares the rows it returns on `DuckDB` with the expected rows
/// (CS-2.4, E6-5.4).
fn check_rows(
    expect: &Expectation,
    plan: &CompiledPlanDocument,
    inputs: &QueryInputs<'_>,
) -> CaseStatus {
    match compare_rows(expect, plan, inputs) {
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::case_file::parse_case;
    use crate::database::prepare_database;
    use crate::divergences::parse_divergences;
    use crate::fixture::{ModelFixture, read_bytes, security_context_path};
    use crate::load_options::resolve_load_options;
    use crate::load_part::{LoadOutcome, ModelDocuments, check_load};

    fn conformance_directory() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
    }

    /// Runs the query part of a case of the `slice` model, whose files are under `cases/cs/`.
    fn status(case_text: &str) -> CaseStatus {
        status_with(case_text, &Divergences::default())
    }

    /// Runs the query part of a case with the given classified divergences.
    fn status_with(case_text: &str, divergences: &Divergences) -> CaseStatus {
        status_in_model("slice", case_text, divergences)
    }

    /// Runs the query part of a case of the `e0-context` model, whose `billed_share` is a
    /// context metric, a capability outside the 0.6.0 slice (VS-1d).
    fn context_status(case_text: &str) -> CaseStatus {
        status_in_model("e0-context", case_text, &Divergences::default())
    }

    /// Runs the query part of a case of the model `model`; the files the case names are under
    /// `cases/cs/`.
    fn status_in_model(model: &str, case_text: &str, divergences: &Divergences) -> CaseStatus {
        let case = parse_case(case_text).unwrap();
        let model_folder = conformance_directory().join("models").join(model);
        let connection = prepare_database(&ModelFixture::read(&model_folder).unwrap()).unwrap();
        let documents = ModelDocuments::read(&model_folder).unwrap();
        let LoadOutcome::Loaded(handle) = check_load(
            &case,
            &documents,
            &resolve_load_options(case.load_options.as_ref()),
        ) else {
            panic!("the {model} model loads");
        };
        let context_name = case.security_context.as_deref().unwrap();
        let security_context =
            read_bytes(&security_context_path(&model_folder, context_name)).unwrap();
        let case_folder = conformance_directory().join("cases/cs");
        let inputs = QueryInputs {
            case_folder: &case_folder,
            security_context: &security_context,
            connection: &connection,
            divergences,
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

    fn context_case_text(query: &str, expect: &str) -> String {
        format!(
            "id: cs-unit-002\nrules: [VS-4]\nmodel: e0-context\n\
             load_options: {{grant_authenticated_to_all: true}}\nsecurity_context: analyst.json\n\
             query: {{contract: menura.query, contract_version: \"1.1.0\", time_zone: UTC, \
             query: {query}}}\nexpect: {expect}\n"
        )
    }

    /// `billed_share` is a context metric, which is outside the 0.6.0 slice (VS-1d).
    const OUTSIDE_THE_SLICE: &str = "{metrics: [billed, billed_share]}";

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
        let status = context_status(&context_case_text(
            OUTSIDE_THE_SLICE,
            "{outcome: plan, rows: cs-plan-rows-001.csv}",
        ));
        let CaseStatus::Pending(reason) = status else {
            panic!("the case is pending: {status:?}");
        };
        assert!(reason.contains(NOT_YET_SUPPORTED), "{reason}");
    }

    #[test]
    fn vs_4_a_case_that_expects_not_yet_supported_compares_it() {
        let expect = |pointer: &str| {
            format!(
                "{{outcome: error, error: E_NOT_YET_SUPPORTED, location: {{document: query, \
                 pointer: {pointer}}}}}"
            )
        };
        assert_eq!(
            context_status(&context_case_text(
                OUTSIDE_THE_SLICE,
                &expect("/query/metrics/1")
            )),
            CaseStatus::Passed
        );
        assert_fails(
            context_status(&context_case_text(
                OUTSIDE_THE_SLICE,
                &expect("/query/metrics/0"),
            )),
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
    fn cs_2_4c_a_runtime_error_case_fails_when_the_plan_executes() {
        let expect = "{outcome: runtime_error, error: RADD-01}";
        assert_fails(status(&case_text(BY_REGION, expect)), "executes");
    }

    #[test]
    fn cs_2_3_a_runtime_error_case_checks_its_golden_file_before_executing() {
        let expect = "{outcome: runtime_error, error: RADD-01, sql: cs-plan-rows-001.csv}";
        assert_fails(status(&case_text(BY_REGION, expect)), "golden");
    }

    /// A differential case of `BY_REGION` whose oracle is the file `oracle`.
    fn differential_case(id: &str, oracle: &str) -> String {
        case_text(BY_REGION, "{outcome: plan, rows: cs-plan-rows-001.csv}")
            .replace("id: cs-unit-001", &format!("id: {id}\noracle: {oracle}"))
    }

    #[test]
    fn cs_4_7_a_differential_case_passes_when_its_rows_match_the_oracle() {
        let text = differential_case("cs-unit-001", "cs-unit-oracle-001.oracle.sql");
        assert_eq!(status(&text), CaseStatus::Passed);
    }

    #[test]
    fn cs_4_7_an_unclassified_divergence_fails_the_case() {
        let text = differential_case("cs-unit-divergent-001", "cs-unit-divergent-001.oracle.sql");
        assert_fails(status(&text), "does not classify");
    }

    #[test]
    fn cs_4_8_a_classified_divergence_leaves_the_case_pending() {
        let text = differential_case("cs-unit-divergent-001", "cs-unit-divergent-001.oracle.sql");
        let divergences = parse_divergences(
            "divergences:\n  - {case: cs-unit-divergent-001, classification: bug, reason: r}\n",
        )
        .unwrap();
        let status = status_with(&text, &divergences);
        assert!(matches!(status, CaseStatus::Pending(_)), "{status:?}");
    }

    #[test]
    fn cs_4_8_a_differential_case_without_its_oracle_fails() {
        let text = differential_case("cs-unit-001", "cs-unit-no-oracle-001.oracle.sql");
        assert_fails(status(&text), "cs-unit-no-oracle-001.oracle.sql");
    }

    #[test]
    fn cs_2_16_explain_matches_the_compiled_plan_of_a_plan_case() {
        let model_folder = conformance_directory().join("models/slice");
        let documents = ModelDocuments::read(&model_folder).unwrap();
        let case = parse_case(&case_text(BY_REGION, "{outcome: plan, rows: e.csv}")).unwrap();
        let LoadOutcome::Loaded(handle) =
            check_load(&case, &documents, &resolve_load_options(None))
        else {
            panic!("the slice model loads");
        };
        let context = read_bytes(&security_context_path(&model_folder, "auditor.json")).unwrap();
        let query = serde_json::to_vec(case.query.as_ref().unwrap()).unwrap();
        let options = CompileOptions::default();
        let CompileResult::Plan(plan) = compile(&handle, &query, &context, &options) else {
            panic!("the query compiles");
        };
        let explained = explain(&handle, &query, &context, &options);
        assert_eq!(explained_mismatch(&plan, &explained), None);
    }

    #[test]
    fn cs_2_4b_an_expected_warning_must_be_reported_by_the_compilation() {
        let expect = "{outcome: plan, rows: cs-plan-rows-001.csv, warnings: [WQRY-01]}";
        assert_fails(status(&case_text(BY_REGION, expect)), "WQRY-01");
    }
}

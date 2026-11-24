//! The `ClickHouse` part of a `plan` or `runtime_error` case (CS-2.3a, E6-7.10): the query is
//! compiled with `dialect: clickhouse`, its SQL is compared with the golden `ClickHouse` SQL, and
//! the plan is executed on the case's session.

use std::path::Path;

use menura_core::{CompileResult, CompiledPlanDocument, Handle, compile, parse_dialect};

use super::parameters::parameter_texts;
use super::results::result_cells;
use super::server::{ServerFailure, Session};
use crate::case::{Case, ClickhouseExpectation, Outcome};
use crate::compile_options::compile_options;
use crate::csv_rows::parse_csv;
use crate::diagnostic_check::describe;
use crate::fixture::read_text;
use crate::inlined_values::inlined_value_mismatch;
use crate::report::CaseStatus;
use crate::row_comparison::{Comparison, cell_rows_mismatch};

/// The code compilation returns when the plan needs a value `ClickHouse` cannot express
/// (E6-7.12).
const INEXPRESSIBLE: &str = "E6-01";

/// What the `ClickHouse` part of a case reads besides the case and the loaded model.
pub(crate) struct ClickhouseInputs<'inputs> {
    /// The folder of the case file, where its `sql_clickhouse` and `rows` files are.
    pub(crate) case_folder: &'inputs Path,
    /// The security context document, as bytes.
    pub(crate) security_context: &'inputs [u8],
    /// The case's session, holding its data.
    pub(crate) session: &'inputs Session<'inputs>,
}

/// Compiles the query of `case` with `dialect: clickhouse` and checks the result (CS-2.3a): a
/// case with `clickhouse: e6_01` passes only when compilation returns `E6-01`; any other case
/// must compile, match its golden `ClickHouse` SQL, and return its rows, or for `runtime_error`
/// fail with its code (CS-2.4c, E6-7.5).
pub(crate) fn check_on_clickhouse(
    case: &Case,
    handle: &Handle,
    inputs: &ClickhouseInputs<'_>,
) -> CaseStatus {
    let result = match compile_for_clickhouse(case, handle, inputs.security_context) {
        Ok(result) => result,
        Err(reason) => return CaseStatus::Failed(reason),
    };
    match (case.expect.clickhouse, result) {
        (Some(ClickhouseExpectation::Inexpressible), CompileResult::Error(error)) => {
            let first = error.errors.first();
            if first.is_some_and(|diagnostic| diagnostic.code == INEXPRESSIBLE) {
                CaseStatus::Passed
            } else {
                CaseStatus::Failed(format!(
                    "on ClickHouse the first error is {}, expected {INEXPRESSIBLE}",
                    describe(first)
                ))
            }
        }
        (Some(ClickhouseExpectation::Inexpressible), CompileResult::Plan(_)) => CaseStatus::Failed(
            format!("the query compiles for ClickHouse, but the case expects {INEXPRESSIBLE}"),
        ),
        (None, CompileResult::Error(error)) => CaseStatus::Failed(format!(
            "the query does not compile for ClickHouse: {}",
            describe(error.errors.first())
        )),
        (None, CompileResult::Plan(plan)) => check_plan(case, &plan, inputs),
    }
}

/// Compiles the query of `case` with its compile options and `dialect: clickhouse`.
fn compile_for_clickhouse(
    case: &Case,
    handle: &Handle,
    security_context: &[u8],
) -> Result<CompileResult, String> {
    let query = case
        .query
        .as_ref()
        .ok_or_else(|| "this outcome requires `query`".to_owned())?;
    let query =
        serde_json::to_vec(query).map_err(|error| format!("the query is not JSON: {error}"))?;
    let mut options = compile_options(case);
    options.dialect = parse_dialect("clickhouse")
        .map_err(|error| format!("core has no dialect `clickhouse`: {error:?}"))?;
    Ok(compile(handle, &query, security_context, &options).result)
}

/// Checks a `ClickHouse` plan: its golden SQL (CS-2.3a), that no value is written in its SQL
/// (E6-4.2), and its execution. The plan is executed even when its golden SQL is missing or
/// differs, and both results are reported.
fn check_plan(
    case: &Case,
    plan: &CompiledPlanDocument,
    inputs: &ClickhouseInputs<'_>,
) -> CaseStatus {
    let golden = golden_mismatch(case, plan, inputs.case_folder);
    if let Some(reason) = inlined_value_mismatch(case.query.as_ref(), inputs.security_context, plan)
    {
        return CaseStatus::Failed(reason);
    }
    let executed = match case.expect.outcome {
        Outcome::RuntimeError => runtime_error_mismatch(case, plan, inputs.session),
        _ => rows_mismatch(case, plan, inputs),
    };
    match (golden, executed) {
        (None, None) => CaseStatus::Passed,
        (Some(golden), None) => CaseStatus::Failed(format!("{golden}; its execution passes")),
        (None, Some(executed)) => CaseStatus::Failed(executed),
        (Some(golden), Some(executed)) => CaseStatus::Failed(format!("{golden}; {executed}")),
    }
}

/// Why the SQL differs from the golden `ClickHouse` SQL file, or why the case has none, or `None`
/// (CS-2.3a). A `runtime_error` case may omit the file, as it may omit `sql` (CS-2.3).
fn golden_mismatch(case: &Case, plan: &CompiledPlanDocument, case_folder: &Path) -> Option<String> {
    let Some(file_name) = case.expect.sql_clickhouse.as_ref() else {
        return (case.expect.outcome == Outcome::Plan).then(|| {
            format!(
                "an `outcome: plan` case names its golden ClickHouse SQL with `sql_clickhouse` \
                 or sets `clickhouse: e6_01` (CS-2.3a); the SQL is:\n{}",
                plan.sql
            )
        });
    };
    match read_text(&case_folder.join(file_name)) {
        Ok(golden) if golden == plan.sql => None,
        Ok(_) => Some(format!(
            "the SQL differs from the golden file `{file_name}`; the SQL is:\n{}",
            plan.sql
        )),
        Err(error) => Some(error.to_string()),
    }
}

/// Executes the plan and compares its rows with the `rows` file of the case (CS-2.4).
fn rows_mismatch(
    case: &Case,
    plan: &CompiledPlanDocument,
    inputs: &ClickhouseInputs<'_>,
) -> Option<String> {
    let compared = || -> Result<Option<String>, String> {
        let file_name = case
            .expect
            .rows
            .as_deref()
            .ok_or_else(|| "`outcome: plan` requires `rows`".to_owned())?;
        let text =
            read_text(&inputs.case_folder.join(file_name)).map_err(|error| error.to_string())?;
        let expected =
            parse_csv(&text).map_err(|reason| format!("the rows file `{file_name}`: {reason}"))?;
        let output = execute(plan, inputs.session)?;
        let actual = result_cells(&plan.result_schema, &output)?;
        Ok(cell_rows_mismatch(
            &plan.result_schema,
            &actual,
            &expected,
            Comparison::of(&case.expect),
        )
        .map(|reason| format!("on ClickHouse {reason}")))
    };
    compared().unwrap_or_else(Some)
}

/// Executes the plan of a `runtime_error` case, which must fail with an error whose message
/// contains the expected code (CS-2.4c, E6-7.5).
fn runtime_error_mismatch(
    case: &Case,
    plan: &CompiledPlanDocument,
    session: &Session<'_>,
) -> Option<String> {
    let code = case.expect.error.as_deref().unwrap_or("none");
    let parameters = match parameter_texts(&plan.parameters) {
        Ok(parameters) => parameters,
        Err(reason) => return Some(format!("the plan cannot be bound: {reason}")),
    };
    match session.execute(&plan.sql, &parameters) {
        Err(ServerFailure::Engine(message)) if message.contains(code) => None,
        Err(failure) => Some(format!(
            "the plan fails on ClickHouse without the runtime code {code}: {}",
            failure.describe()
        )),
        Ok(_) => Some(format!(
            "the plan executes on ClickHouse, but the case expects the runtime code {code}"
        )),
    }
}

/// The `TabSeparated` output of the plan's SQL, with its parameters bound (E6-7.3).
fn execute(plan: &CompiledPlanDocument, session: &Session<'_>) -> Result<String, String> {
    let parameters = parameter_texts(&plan.parameters)
        .map_err(|reason| format!("the plan cannot be bound: {reason}"))?;
    session
        .execute(&plan.sql, &parameters)
        .map_err(|failure| format!("the plan fails on ClickHouse: {}", failure.describe()))
}

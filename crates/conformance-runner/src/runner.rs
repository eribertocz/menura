//! Finds the cases of a conformance directory (CS-1), runs each one (CS-4.1, CS-4.2) and
//! collects the report (CS-4.3). A compile-only run checks every case without executing any
//! plan, and needs no `DuckDB` (PLT-1.2).

#[cfg(feature = "duckdb")]
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "duckdb")]
use duckdb::Connection;
use menura_core::LoadOptions;

use crate::case::{Case, Outcome};
use crate::case_file::read_case;
#[cfg(feature = "duckdb")]
use crate::database::prepare_database;
use crate::describe_part::check_describe;
#[cfg(feature = "duckdb")]
use crate::divergences::{DIFFERENTIAL_FOLDER, DIVERGENCES_FILE, Divergences, read_divergences};
use crate::error::{RunnerError, RunnerErrorKind};
#[cfg(feature = "duckdb")]
use crate::fixture::ModelFixture;
use crate::fixture::{read_bytes, security_context_path};
use crate::integrity_checks_part::integrity_checks_part;
use crate::load_options::resolve_load_options;
use crate::load_part::{LoadOutcome, ModelDocuments, check_load};
#[cfg(feature = "duckdb")]
use crate::ossie::{OSSIE_AREA, check_layout, run_suites};
#[cfg(feature = "duckdb")]
use crate::plan_execution::CaseDatabase;
use crate::query_part::{QueryInputs, check_query};
use crate::report::{CaseResult, CaseStatus, Report};

/// How the cases of a run are checked.
enum Checks {
    /// Every plan is executed on the case's fresh database, with the reviewed divergences of
    /// the differential cases (CS-4.2, CS-4.8, E6-5.4).
    #[cfg(feature = "duckdb")]
    Executed(Divergences),
    /// Every case is loaded, compiled and compared with its golden files, but no plan is
    /// executed and no database is prepared (PLT-1.2).
    CompileOnly,
}

/// Runs every case of `conformance_directory/cases/`, or only those of `area` (CS-4.1).
///
/// Without an area, the layout of `ossie/` is also checked (the stale-list rule of OSS-2.4), but
/// its suite cases are not run: their outcomes are judged only by the area `ossie`, which runs
/// the Apache Ossie suites alone (OSS-2.3).
///
/// A case whose fixtures cannot be read or whose database cannot be prepared is reported as
/// failed. Otherwise its load part runs through `core` (CS-2.8): a `load_error` case passes or
/// fails; any other case fails its load part, or its query part is compiled for its security
/// context, and a plan is executed on the case's database.
///
/// # Errors
///
/// Returns a [`RunnerError`] when the layout cannot be read, the area does not exist, a case
/// file breaks CS-1 or CS-2, or the layout of `ossie/` breaks OSS-2.4 or CS-4.5.
#[cfg(feature = "duckdb")]
pub fn run(conformance_directory: &Path, area: Option<&str>) -> Result<Report, RunnerError> {
    let ossie_directory = conformance_directory.join(OSSIE_AREA);
    let mut report = Report::default();
    match area {
        Some(OSSIE_AREA) if !ossie_directory.is_dir() => {
            return Err(RunnerError::new(
                RunnerErrorKind::UnknownArea(OSSIE_AREA.to_owned()),
                conformance_directory,
            ));
        }
        Some(OSSIE_AREA) => report.cases = run_suites(&ossie_directory)?,
        None if ossie_directory.is_dir() => {
            check_layout(&ossie_directory)?;
            report.cases = run_cases(conformance_directory, area)?;
        }
        _ => report.cases = run_cases(conformance_directory, area)?,
    }
    Ok(report)
}

/// Checks every case of `conformance_directory/cases/` as `run` does, but executes no plan:
/// each case is loaded and compiled through `core`, and every output it gives (SQL, plan,
/// error, description and check documents) is compared with its golden files (PLT-1.2). A case
/// that only execution decides (its rows, its runtime error, its oracle) passes once its plan
/// meets every check made before execution. The Apache Ossie suites, which execute their
/// queries, are not run.
///
/// # Errors
///
/// Returns a [`RunnerError`] when the layout cannot be read or a case file breaks CS-1 or CS-2.
pub fn run_compile_only(conformance_directory: &Path) -> Result<Report, RunnerError> {
    Ok(Report {
        cases: check_cases(conformance_directory, None, &Checks::CompileOnly)?,
    })
}

/// Runs the cases of `cases/`, or only those of `area`, executing their plans.
#[cfg(feature = "duckdb")]
fn run_cases(
    conformance_directory: &Path,
    area: Option<&str>,
) -> Result<Vec<CaseResult>, RunnerError> {
    let divergences = read_divergences(conformance_directory)?;
    check_divergence_cases(conformance_directory, &divergences)?;
    check_cases(conformance_directory, area, &Checks::Executed(divergences))
}

/// Checks the cases of `cases/`, or only those of `area`, with `checks`.
fn check_cases(
    conformance_directory: &Path,
    area: Option<&str>,
    checks: &Checks,
) -> Result<Vec<CaseResult>, RunnerError> {
    let mut results = Vec::new();
    for area_folder in area_folders(&conformance_directory.join("cases"), area)? {
        for case_path in case_files(&area_folder)? {
            let case = read_case(&case_path)?;
            let status = run_case(conformance_directory, &area_folder, &case, checks);
            results.push(CaseResult {
                id: case.id,
                rules: case.rules,
                status,
            });
        }
    }
    Ok(results)
}

/// Checks that every entry of the reviewed file of divergences names a differential case of
/// `cases/`, whichever area runs (CS-4.8).
#[cfg(feature = "duckdb")]
fn check_divergence_cases(
    conformance_directory: &Path,
    divergences: &Divergences,
) -> Result<(), RunnerError> {
    let cases_directory = conformance_directory.join("cases");
    let mut differential = BTreeSet::new();
    for area_folder in area_folders(&cases_directory, None)? {
        for case_path in case_files(&area_folder)? {
            let case = read_case(&case_path)?;
            if case.oracle.is_some() {
                differential.insert(case.id);
            }
        }
    }
    match divergences
        .cases()
        .find(|case| !differential.contains(*case))
    {
        Some(case) => Err(RunnerError::new(
            RunnerErrorKind::UnknownDivergence(case.to_string()),
            conformance_directory
                .join(DIFFERENTIAL_FOLDER)
                .join(DIVERGENCES_FILE),
        )),
        None => Ok(()),
    }
}

fn run_case(
    conformance_directory: &Path,
    case_folder: &Path,
    case: &Case,
    checks: &Checks,
) -> CaseStatus {
    let model_folder = conformance_directory.join("models").join(&case.model);
    let prepared = prepare_case(conformance_directory, case, checks).and_then(|prepared| {
        ModelDocuments::read(&model_folder).map(|documents| (prepared, documents))
    });
    let (prepared, documents) = match prepared {
        Ok(prepared) => prepared,
        Err(error) => return CaseStatus::Failed(error.to_string()),
    };
    match check_load(case, &documents, &prepared.load_options) {
        LoadOutcome::Checked(status) => status,
        LoadOutcome::Loaded(handle) if case.expect.outcome == Outcome::IntegrityChecks => {
            integrity_checks_part(case, &handle, case_folder)
        }
        LoadOutcome::Loaded(handle) => {
            let Some(security_context) = prepared.security_context.as_deref() else {
                return CaseStatus::Failed("this outcome requires `security_context`".to_owned());
            };
            if case.expect.outcome == Outcome::Describe {
                return check_describe(case, &handle, security_context, case_folder);
            }
            let inputs = QueryInputs {
                case_folder,
                security_context,
                #[cfg(feature = "duckdb")]
                database: case_database(checks, &prepared),
            };
            check_query(case, &handle, &inputs)
        }
    }
}

/// What a case runs with once its fixtures are read and, when its plan is executed, its
/// database is prepared.
struct PreparedCase {
    /// The load options its model is loaded with (CS-2.7).
    load_options: LoadOptions,
    /// Its security context document, as bytes, when it names one.
    security_context: Option<Vec<u8>>,
    /// Its fresh database (CS-4.2), when its plan is executed.
    #[cfg(feature = "duckdb")]
    connection: Option<Connection>,
}

/// Reads the security context of `case` and, when `checks` executes plans, checks its fixtures
/// and prepares its database.
#[cfg_attr(
    not(feature = "duckdb"),
    expect(
        unused_variables,
        reason = "without the `duckdb` feature every run is compile-only: no database is prepared"
    )
)]
fn prepare_case(
    conformance_directory: &Path,
    case: &Case,
    checks: &Checks,
) -> Result<PreparedCase, RunnerError> {
    let model_folder = conformance_directory.join("models").join(&case.model);
    #[cfg(feature = "duckdb")]
    let connection = prepared_database(checks, &model_folder)?;
    let security_context = case
        .security_context
        .as_ref()
        .map(|file_name| read_bytes(&security_context_path(&model_folder, file_name)))
        .transpose()?;
    Ok(PreparedCase {
        load_options: resolve_load_options(case.load_options.as_ref()),
        security_context,
        #[cfg(feature = "duckdb")]
        connection,
    })
}

/// The fresh database of a case of the model in `model_folder` when `checks` executes plans
/// (CS-4.2), after its fixtures are checked; `None` for a compile-only run.
#[cfg(feature = "duckdb")]
fn prepared_database(
    checks: &Checks,
    model_folder: &Path,
) -> Result<Option<Connection>, RunnerError> {
    match checks {
        Checks::Executed(_) => {
            let fixture = ModelFixture::read(model_folder)?;
            prepare_database(&fixture)
                .map(Some)
                .map_err(|error| RunnerError::new(RunnerErrorKind::Database(error), model_folder))
        }
        Checks::CompileOnly => Ok(None),
    }
}

/// What the plan of a prepared case is executed with, or `None` for a compile-only run.
#[cfg(feature = "duckdb")]
fn case_database<'run>(
    checks: &'run Checks,
    prepared: &'run PreparedCase,
) -> Option<CaseDatabase<'run>> {
    match (checks, &prepared.connection) {
        (Checks::Executed(divergences), Some(connection)) => Some(CaseDatabase {
            connection,
            divergences,
        }),
        _ => None,
    }
}

/// The area folders to run, sorted by name.
pub(crate) fn area_folders(
    cases_directory: &Path,
    area: Option<&str>,
) -> Result<Vec<PathBuf>, RunnerError> {
    if let Some(area) = area {
        let folder = cases_directory.join(area);
        if !folder.is_dir() {
            return Err(RunnerError::new(
                RunnerErrorKind::UnknownArea(area.to_owned()),
                cases_directory,
            ));
        }
        return Ok(vec![folder]);
    }
    let mut folders = list_directory(cases_directory)?;
    folders.retain(|path| path.is_dir());
    Ok(folders)
}

/// The `.yaml` case files of an area folder, sorted by name.
pub(crate) fn case_files(area_folder: &Path) -> Result<Vec<PathBuf>, RunnerError> {
    let mut files = list_directory(area_folder)?;
    files.retain(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "yaml"));
    Ok(files)
}

/// The entries of a directory, sorted by path so that runs are deterministic.
pub(crate) fn list_directory(directory: &Path) -> Result<Vec<PathBuf>, RunnerError> {
    let unreadable = |error| RunnerError::new(RunnerErrorKind::Unreadable(error), directory);
    let mut paths = Vec::new();
    for entry in fs::read_dir(directory).map_err(unreadable)? {
        paths.push(entry.map_err(unreadable)?.path());
    }
    paths.sort();
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use menura_core::OssieFormat;

    use super::*;
    use crate::case_file::parse_case;

    const LOAD_ERROR_CASE: &str = "
id: sample-load-options-001
rules: [CS-2.7]
model: sample
expect: {outcome: load_error, error: E0D-02}
";

    fn conformance_directory() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
    }

    #[test]
    fn cs_2_7_the_load_options_of_a_case_reach_the_load_call() {
        let text = LOAD_ERROR_CASE.replace(
            "model: sample",
            "model: sample\nload_options: {ossie_format: development}",
        );
        let case = parse_case(&text).unwrap();
        let options = prepare_case(&conformance_directory(), &case, &Checks::CompileOnly)
            .unwrap()
            .load_options;
        assert_eq!(options.ossie_format, OssieFormat::Development);
        assert!(!options.grant_authenticated_to_all);
        assert_eq!(options.namespace, "menura");
    }

    #[test]
    fn cs_2_7_a_case_without_load_options_is_loaded_with_the_defaults() {
        let case = parse_case(LOAD_ERROR_CASE).unwrap();
        let options = prepare_case(&conformance_directory(), &case, &Checks::CompileOnly)
            .unwrap()
            .load_options;
        assert_eq!(options, resolve_load_options(None));
    }

    /// A `plan` case of the `slice` model whose files are under `cases/cs/`, expecting `expect`.
    fn plan_case(expect: &str) -> Case {
        parse_case(&format!(
            "id: cs-unit-001\nrules: [CS-2.8]\nmodel: slice\nsecurity_context: auditor.json\n\
             query: {{contract: menura.query, contract_version: \"1.1.0\", time_zone: UTC, \
             query: {{dimensions: [sales.region], metrics: [total_amount]}}}}\nexpect: {expect}\n"
        ))
        .unwrap()
    }

    fn compile_only_status(case: &Case) -> CaseStatus {
        let directory = conformance_directory();
        run_case(
            &directory,
            &directory.join("cases/cs"),
            case,
            &Checks::CompileOnly,
        )
    }

    #[test]
    fn plt_1_2_a_compile_only_run_compares_the_sql_with_its_golden_file() {
        let case =
            plan_case("{outcome: plan, sql: cs-plan-rows-001.csv, rows: cs-plan-rows-001.csv}");
        let CaseStatus::Failed(reason) = compile_only_status(&case) else {
            panic!("the SQL differs from the golden file");
        };
        assert!(reason.contains("golden"), "{reason}");
    }

    #[test]
    fn plt_1_2_a_compile_only_run_executes_no_plan() {
        for expect in [
            "{outcome: plan, rows: e1-output-name-001.csv}",
            "{outcome: runtime_error, error: RADD-01}",
        ] {
            assert_eq!(compile_only_status(&plan_case(expect)), CaseStatus::Passed);
        }
    }

    #[test]
    fn plt_1_2_a_compile_only_run_still_fails_a_query_that_does_not_compile() {
        let case = plan_case("{outcome: error, error: E_NAME_NOT_FOUND}");
        assert!(matches!(compile_only_status(&case), CaseStatus::Failed(_)));
    }

    #[test]
    fn plt_1_2_a_compile_only_run_checks_every_case_of_the_suite() {
        let directory = conformance_directory();
        let report = run_compile_only(&directory).unwrap();
        let not_passed: Vec<&CaseResult> = report
            .cases
            .iter()
            .filter(|case| case.status != CaseStatus::Passed)
            .collect();
        assert!(not_passed.is_empty(), "{not_passed:?}");
        let mut case_count = 0;
        for area_folder in area_folders(&directory.join("cases"), None).unwrap() {
            case_count += case_files(&area_folder).unwrap().len();
        }
        assert_eq!(report.cases.len(), case_count);
    }
}

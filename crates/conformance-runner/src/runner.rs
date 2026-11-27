//! Finds the cases of a conformance directory (CS-1), runs each one (CS-4.1, CS-4.2) and
//! collects the report (CS-4.3).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use duckdb::Connection;
use menura_core::LoadOptions;

use crate::case::{Case, Outcome};
use crate::case_file::read_case;
use crate::database::prepare_database;
use crate::describe_part::check_describe;
use crate::divergences::{DIFFERENTIAL_FOLDER, DIVERGENCES_FILE, Divergences, read_divergences};
use crate::error::{RunnerError, RunnerErrorKind};
use crate::fixture::{ModelFixture, read_bytes, security_context_path};
use crate::integrity_checks_part::integrity_checks_part;
use crate::load_options::resolve_load_options;
use crate::load_part::{LoadOutcome, ModelDocuments, check_load};
use crate::ossie::{OSSIE_AREA, check_layout, run_suites};
use crate::plan_execution::CaseDatabase;
use crate::query_part::{QueryInputs, check_query};
use crate::report::{CaseResult, CaseStatus, Report};

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

/// Runs the cases of `cases/`, or only those of `area`.
fn run_cases(
    conformance_directory: &Path,
    area: Option<&str>,
) -> Result<Vec<CaseResult>, RunnerError> {
    let divergences = read_divergences(conformance_directory)?;
    check_divergence_cases(conformance_directory, &divergences)?;
    let mut results = Vec::new();
    for area_folder in area_folders(&conformance_directory.join("cases"), area)? {
        for case_path in case_files(&area_folder)? {
            let case = read_case(&case_path)?;
            let status = run_case(conformance_directory, &area_folder, &case, &divergences);
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
    divergences: &Divergences,
) -> CaseStatus {
    let model_folder = conformance_directory.join("models").join(&case.model);
    let prepared = prepare_case(conformance_directory, case).and_then(|prepared| {
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
                database: CaseDatabase {
                    connection: &prepared.connection,
                    divergences,
                },
            };
            check_query(case, &handle, &inputs)
        }
    }
}

/// What a case runs with once its fixtures are read and its database is prepared.
struct PreparedCase {
    /// The load options its model is loaded with (CS-2.7).
    load_options: LoadOptions,
    /// Its security context document, as bytes, when it names one.
    security_context: Option<Vec<u8>>,
    /// Its fresh database (CS-4.2).
    connection: Connection,
}

/// Checks the fixtures of `case`, reads its security context and prepares its database.
fn prepare_case(conformance_directory: &Path, case: &Case) -> Result<PreparedCase, RunnerError> {
    let model_folder = conformance_directory.join("models").join(&case.model);
    let fixture = ModelFixture::read(&model_folder)?;
    let security_context = case
        .security_context
        .as_ref()
        .map(|file_name| read_bytes(&security_context_path(&model_folder, file_name)))
        .transpose()?;
    let connection = prepare_database(&fixture)
        .map_err(|error| RunnerError::new(RunnerErrorKind::Database(error), &model_folder))?;
    Ok(PreparedCase {
        load_options: resolve_load_options(case.load_options.as_ref()),
        security_context,
        connection,
    })
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
        let options = prepare_case(&conformance_directory(), &case)
            .unwrap()
            .load_options;
        assert_eq!(options.ossie_format, OssieFormat::Development);
        assert!(!options.grant_authenticated_to_all);
        assert_eq!(options.namespace, "menura");
    }

    #[test]
    fn cs_2_7_a_case_without_load_options_is_loaded_with_the_defaults() {
        let case = parse_case(LOAD_ERROR_CASE).unwrap();
        let options = prepare_case(&conformance_directory(), &case)
            .unwrap()
            .load_options;
        assert_eq!(options, resolve_load_options(None));
    }
}

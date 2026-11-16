//! Finds the cases of a conformance directory (CS-1), runs each one (CS-4.1, CS-4.2) and
//! collects the report (CS-4.3).

use std::fs;
use std::path::{Path, PathBuf};

use crate::case::Case;
use crate::case_file::read_case;
use crate::database::prepare_database;
use crate::error::{RunnerError, RunnerErrorKind};
use crate::fixture::{ModelFixture, security_context_path};
use crate::ossie::{OSSIE_AREA, run_suites};
use crate::report::{CaseResult, CaseStatus, Report};

/// Why every case is pending in this version: no compilation stage exists yet.
pub const PENDING_REASON: &str = "the compiler stages are not implemented yet";

/// Runs every case of `conformance_directory`, or only those of `area` (CS-4.1). Every case
/// includes the Apache Ossie suites of `ossie/` (CS-3.1), which the area `ossie` selects alone.
///
/// A case whose fixtures cannot be read or whose database cannot be prepared is reported as
/// failed; otherwise it is reported as pending, because no compilation stage exists yet. The
/// Apache Ossie cases are also checked against `ossie/expected-failures.yaml` (OSS-2.4).
///
/// # Errors
///
/// Returns a [`RunnerError`] when the layout cannot be read, the area does not exist, or a case
/// file breaks CS-1 or CS-2.
pub fn run(conformance_directory: &Path, area: Option<&str>) -> Result<Report, RunnerError> {
    let mut report = Report::default();
    if area != Some(OSSIE_AREA) {
        report.cases = run_cases(conformance_directory, area)?;
    }
    let ossie_directory = conformance_directory.join(OSSIE_AREA);
    match area {
        Some(OSSIE_AREA) if !ossie_directory.is_dir() => {
            return Err(RunnerError::new(
                RunnerErrorKind::UnknownArea(OSSIE_AREA.to_owned()),
                conformance_directory,
            ));
        }
        Some(OSSIE_AREA) | None if ossie_directory.is_dir() => {
            report.cases.extend(run_suites(&ossie_directory)?);
        }
        _ => {}
    }
    Ok(report)
}

/// Runs the cases of `cases/`, or only those of `area`.
fn run_cases(
    conformance_directory: &Path,
    area: Option<&str>,
) -> Result<Vec<CaseResult>, RunnerError> {
    let mut results = Vec::new();
    for area_folder in area_folders(&conformance_directory.join("cases"), area)? {
        for case_path in case_files(&area_folder)? {
            let case = read_case(&case_path)?;
            let status = run_case(conformance_directory, &case);
            results.push(CaseResult {
                id: case.id,
                rules: case.rules,
                status,
            });
        }
    }
    Ok(results)
}

fn run_case(conformance_directory: &Path, case: &Case) -> CaseStatus {
    match prepare_case(conformance_directory, case) {
        Ok(()) => CaseStatus::Pending(PENDING_REASON.to_owned()),
        Err(error) => CaseStatus::Failed(error.to_string()),
    }
}

fn prepare_case(conformance_directory: &Path, case: &Case) -> Result<(), RunnerError> {
    let model_folder = conformance_directory.join("models").join(&case.model);
    let fixture = ModelFixture::read(&model_folder)?;
    if let Some(file_name) = &case.security_context {
        let path = security_context_path(&model_folder, file_name);
        fs::metadata(&path)
            .map_err(|error| RunnerError::new(RunnerErrorKind::Unreadable(error), &path))?;
    }
    prepare_database(&fixture)
        .map_err(|error| RunnerError::new(RunnerErrorKind::Database(error), &model_folder))?;
    Ok(())
}

/// The area folders to run, sorted by name.
fn area_folders(cases_directory: &Path, area: Option<&str>) -> Result<Vec<PathBuf>, RunnerError> {
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
fn case_files(area_folder: &Path) -> Result<Vec<PathBuf>, RunnerError> {
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

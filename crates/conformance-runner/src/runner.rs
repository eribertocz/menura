//! Finds the cases of a conformance directory (CS-1), runs each one (CS-4.1, CS-4.2) and
//! collects the report (CS-4.3).

use std::fs;
use std::path::{Path, PathBuf};

use semantic_compiler_core::LoadOptions;

use crate::case::Case;
use crate::case_file::read_case;
use crate::database::prepare_database;
use crate::error::{RunnerError, RunnerErrorKind};
use crate::fixture::{ModelFixture, security_context_path};
use crate::load_options::resolve_load_options;
pub use crate::load_part::QUERY_PENDING_REASON;
use crate::load_part::{ModelDocuments, check_load};
use crate::ossie::{OSSIE_AREA, check_layout, run_suites};
use crate::report::{CaseResult, CaseStatus, Report};

/// Runs every case of `conformance_directory/cases/`, or only those of `area` (CS-4.1).
///
/// Without an area, the layout of `ossie/` is also checked (the stale-list rule of OSS-2.4), but
/// its suite cases are not run: their outcomes are judged only by the area `ossie`, which runs
/// the Apache Ossie suites alone (OSS-2.3).
///
/// A case whose fixtures cannot be read or whose database cannot be prepared is reported as
/// failed. Otherwise its load part runs through `core` (CS-2.8): a `load_error` case passes or
/// fails; any other case fails its load part or is reported as pending, because its query
/// part needs stages E1 to E6.
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
    let model_folder = conformance_directory.join("models").join(&case.model);
    let prepared = prepare_case(conformance_directory, case).and_then(|options| {
        ModelDocuments::read(&model_folder).map(|documents| (options, documents))
    });
    match prepared {
        Ok((options, documents)) => check_load(case, &documents, &options),
        Err(error) => CaseStatus::Failed(error.to_string()),
    }
}

/// Checks the fixtures of `case`, prepares its database and returns the load options its model
/// is loaded with (CS-2.7).
fn prepare_case(conformance_directory: &Path, case: &Case) -> Result<LoadOptions, RunnerError> {
    let model_folder = conformance_directory.join("models").join(&case.model);
    let fixture = ModelFixture::read(&model_folder)?;
    if let Some(file_name) = &case.security_context {
        let path = security_context_path(&model_folder, file_name);
        fs::metadata(&path)
            .map_err(|error| RunnerError::new(RunnerErrorKind::Unreadable(error), &path))?;
    }
    prepare_database(&fixture)
        .map_err(|error| RunnerError::new(RunnerErrorKind::Database(error), &model_folder))?;
    Ok(resolve_load_options(case.load_options.as_ref()))
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

#[cfg(test)]
mod tests {
    use semantic_compiler_core::OssieFormat;

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
        let options = prepare_case(&conformance_directory(), &case).unwrap();
        assert_eq!(options.ossie_format, OssieFormat::Development);
        assert!(!options.grant_authenticated_to_all);
        assert_eq!(options.namespace, "semantic_compiler");
    }

    #[test]
    fn cs_2_7_a_case_without_load_options_is_loaded_with_the_defaults() {
        let case = parse_case(LOAD_ERROR_CASE).unwrap();
        let options = prepare_case(&conformance_directory(), &case).unwrap();
        assert_eq!(options, resolve_load_options(None));
    }
}

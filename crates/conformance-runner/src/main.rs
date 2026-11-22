//! Command line of the conformance runner:
//! `conformance-runner <conformance-directory> [area]`, or
//! `conformance-runner <conformance-directory> --regenerate <case-id>...` (CS-2.10a).

#![forbid(unsafe_code)]
#![allow(
    clippy::disallowed_methods,
    reason = "A2: like the command-line wrapper, the conformance runner reads its arguments and \
              the `CI` variable"
)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use menura_conformance_runner::case::CaseId;
use menura_conformance_runner::{Regeneration, regenerate, run};

const USAGE: &str = "usage: conformance-runner <conformance-directory> [area]\n       \
                     conformance-runner <conformance-directory> --regenerate <case-id>...";

/// The flag that regenerates the golden files of the cases named after it (CS-2.10a).
const REGENERATE: &str = "--regenerate";

fn main() -> ExitCode {
    let mut arguments = std::env::args_os().skip(1);
    let Some(directory) = arguments.next().map(PathBuf::from) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let rest: Vec<OsString> = arguments.collect();
    match rest.split_first() {
        Some((flag, case_ids)) if flag == REGENERATE => run_regeneration(&directory, case_ids),
        None => run_cases(&directory, None),
        Some((area, [])) => {
            let area = area.to_string_lossy();
            run_cases(
                &directory,
                Some(area.as_ref()).filter(|area| !area.is_empty()),
            )
        }
        Some(_) => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn run_cases(directory: &Path, area: Option<&str>) -> ExitCode {
    match run(directory, area) {
        Ok(report) => {
            print!("{}", report.render());
            if report.has_failures() {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

/// Regenerates the golden files of the named cases only, and never in CI (CS-2.10a).
fn run_regeneration(directory: &Path, case_ids: &[OsString]) -> ExitCode {
    if std::env::var_os("CI").is_some() {
        eprintln!("CS-2.10a: golden files are never regenerated in CI");
        return ExitCode::from(2);
    }
    if case_ids.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let case_ids: Vec<CaseId> = case_ids
        .iter()
        .map(|case_id| CaseId::new(case_id.to_string_lossy().into_owned()))
        .collect();
    match regenerate(directory, &case_ids) {
        Ok(regenerations) => {
            for regeneration in regenerations {
                match regeneration {
                    Regeneration::Written(path) => println!("written {}", path.display()),
                    Regeneration::NotWritten(path, reason) => {
                        println!("not written {}: {reason}", path.display());
                    }
                }
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

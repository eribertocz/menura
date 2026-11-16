//! Command line of the conformance runner: `conformance-runner <conformance-directory> [area]`.

#![forbid(unsafe_code)]
#![allow(
    clippy::disallowed_methods,
    reason = "A2: like the command-line wrapper, the conformance runner reads its arguments"
)]

use std::path::PathBuf;
use std::process::ExitCode;

use semantic_compiler_conformance_runner::run;

const USAGE: &str = "usage: conformance-runner <conformance-directory> [area]";

fn main() -> ExitCode {
    let mut arguments = std::env::args_os().skip(1);
    let Some(directory) = arguments.next().map(PathBuf::from) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let area = arguments
        .next()
        .map(|area| area.to_string_lossy().into_owned());
    if arguments.next().is_some() {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let area = area.filter(|area| !area.is_empty());
    match run(&directory, area.as_deref()) {
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

//! The benchmark binary (R4).
//!
//! - `wall-clock`: the R1 and R2 measures of R6.
//! - `instructions <baseline>`: the Callgrind counts, the R3 sweep and the R7 regression check
//!   against the baseline file.
//! - `write-baseline <baseline>`: the Callgrind counts and the R3 sweep, written to the
//!   baseline file instead of compared with it (R7: only a pull request that names the cause
//!   may do this).
//! - `scenario <name> <compilations>`: one scenario, run under Callgrind by the two commands
//!   above.
//!
//! Exit status `0` when every check passes, `1` when a check fails, `2` when the benchmarks
//! cannot run.

#![forbid(unsafe_code)]
#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "R4: test tooling like the conformance runner; the binary reads its arguments, \
              reads and writes the baseline file and runs Valgrind"
)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use menura_benchmarks::{
    BenchmarkError, InstructionCounts, Report, instruction_report, run_scenario, wall_clock_report,
};

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match run(&arguments) {
        Ok(None) => ExitCode::SUCCESS,
        Ok(Some(report)) => {
            print!("{}", report.render());
            for failure in &report.failures {
                eprintln!("{failure}");
            }
            if report.failures.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn run(arguments: &[String]) -> Result<Option<Report>, BenchmarkError> {
    let words: Vec<&str> = arguments.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["wall-clock"] => wall_clock_report().map(Some),
        ["instructions", baseline] => {
            let text = std::fs::read_to_string(baseline)
                .map_err(|error| BenchmarkError::Io(format!("cannot read {baseline}: {error}")))?;
            let baseline = InstructionCounts::parse(&text)?;
            let (report, _) = count(Some(&baseline))?;
            Ok(Some(report))
        }
        ["write-baseline", baseline] => {
            let (report, counts) = count(None)?;
            std::fs::write(baseline, counts.render())
                .map_err(|error| BenchmarkError::Io(format!("cannot write {baseline}: {error}")))?;
            Ok(Some(report))
        }
        ["scenario", name, compilations] => {
            let compilations = compilations
                .parse()
                .map_err(|_| BenchmarkError::Usage(format!("not a count: {compilations}")))?;
            run_scenario(name, compilations).map(|()| None)
        }
        _ => Err(BenchmarkError::Usage(
            "usage: menura-benchmarks wall-clock | instructions <baseline> | write-baseline <baseline> | scenario <name> <compilations>".to_owned(),
        )),
    }
}

fn count(
    baseline: Option<&InstructionCounts>,
) -> Result<(Report, InstructionCounts), BenchmarkError> {
    let executable = std::env::current_exe().map_err(|error| {
        BenchmarkError::Io(format!("cannot find the benchmark binary: {error}"))
    })?;
    let directory = output_directory()?;
    instruction_report(&executable, &directory, baseline)
}

fn output_directory() -> Result<PathBuf, BenchmarkError> {
    let directory = Path::new("target").join("callgrind");
    std::fs::create_dir_all(&directory).map_err(|error| {
        BenchmarkError::Io(format!("cannot create {}: {error}", directory.display()))
    })?;
    Ok(directory)
}

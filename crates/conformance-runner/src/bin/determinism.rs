//! `determinism <conformance-directory> --work <directory>`: the determinism gate (PLN-2.4,
//! P3). Runs every conformance case twice through `core`, in two separate child processes of
//! this binary, the second with the keys of every input document reordered with a fixed seed,
//! and compares the documents of every call byte for byte, hashes included. Every difference
//! is printed with its case, its run and its first differing line; any difference exits `1`.
//!
//! A child process is `determinism <conformance-directory> --emit <file> [--reorder]`: it
//! writes the documents of every call to `file`.

#![forbid(unsafe_code)]
#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "A2: like the command-line wrapper, the determinism check reads its arguments, \
              writes its runs and starts its own child processes"
)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use menura_conformance_runner::determinism::{RunOutputs, compare_runs, run_calls};

const USAGE: &str = "usage: determinism <conformance-directory> --work <directory>\n       \
                     determinism <conformance-directory> --emit <file> [--reorder]";

/// What the command line asks for.
enum Mode {
    /// Run both children in `work` and compare their documents.
    Check { work: PathBuf },
    /// Write the documents of every call to `file`, reordered when asked.
    Emit { file: PathBuf, reorder: bool },
}

fn main() -> ExitCode {
    let arguments: Vec<OsString> = std::env::args_os().skip(1).collect();
    let Some((directory, mode)) = parse(&arguments) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let result = match mode {
        Mode::Check { work } => check(&directory, &work),
        Mode::Emit { file, reorder } => emit(&directory, &file, reorder).map(|()| true),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("PLN-2.4: {error}");
            ExitCode::from(2)
        }
    }
}

fn parse(arguments: &[OsString]) -> Option<(PathBuf, Mode)> {
    let (directory, flags) = arguments.split_first()?;
    let flags: Vec<&str> = flags
        .iter()
        .map(|flag| flag.to_str())
        .collect::<Option<_>>()?;
    let mode = match flags.as_slice() {
        ["--work", work] => Mode::Check {
            work: PathBuf::from(work),
        },
        ["--emit", file] => Mode::Emit {
            file: PathBuf::from(file),
            reorder: false,
        },
        ["--emit", file, "--reorder"] => Mode::Emit {
            file: PathBuf::from(file),
            reorder: true,
        },
        _ => return None,
    };
    Some((PathBuf::from(directory), mode))
}

/// Writes the documents of every call to `file`.
fn emit(directory: &Path, file: &Path, reorder: bool) -> Result<(), String> {
    let outputs = run_calls(directory, reorder).map_err(|error| error.to_string())?;
    std::fs::write(file, outputs.to_json())
        .map_err(|error| format!("`{}`: {error}", file.display()))
}

/// Runs both children, prints every difference, and tells whether there is none.
fn check(directory: &Path, work: &Path) -> Result<bool, String> {
    std::fs::create_dir_all(work).map_err(|error| format!("`{}`: {error}", work.display()))?;
    let first = run_child(directory, &work.join("run-1.json"), false)?;
    let second = run_child(directory, &work.join("run-2.json"), true)?;
    let comparison = compare_runs(&first, &second);
    for difference in &comparison.differences {
        println!("{difference}");
    }
    println!(
        "{} calls in two processes; run 2 reordered {} of {} input documents; {} calls \
         byte-identical, {} equal up to input order, {} differences",
        first.calls.len(),
        second.reordered,
        second.documents,
        first
            .calls
            .len()
            .saturating_sub(comparison.input_order_only + comparison.differences.len()),
        comparison.input_order_only,
        comparison.differences.len()
    );
    Ok(comparison.differences.is_empty())
}

/// Runs this binary as a child process that writes its documents to `file`, and reads them.
fn run_child(directory: &Path, file: &Path, reorder: bool) -> Result<RunOutputs, String> {
    let program = std::env::current_exe().map_err(|error| error.to_string())?;
    let mut command = Command::new(&program);
    command.arg(directory).arg("--emit").arg(file);
    if reorder {
        command.arg("--reorder");
    }
    let status = command
        .status()
        .map_err(|error| format!("`{}`: {error}", program.display()))?;
    if !status.success() {
        return Err(format!(
            "the run writing `{}` failed: {status}",
            file.display()
        ));
    }
    let text =
        std::fs::read_to_string(file).map_err(|error| format!("`{}`: {error}", file.display()))?;
    RunOutputs::from_json(&text).ok_or_else(|| format!("`{}` is not a run", file.display()))
}

//! The command line of `menura` and of its alias `semantic-compiler` (CON-0.5): it reads its
//! files as bytes, calls `core`, and writes the validation report as JSON on standard output
//! (ARC-5.4, ARC-5.5). Both binaries run this function, so they give the same results.

#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "A2: the command-line wrapper is the only crate allowed to use I/O, the environment and the clock"
)]

use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use crate::arguments::{Command, USAGE, UsageError, ValidateFiles, parse};
use menura_core::{LoadOptions, document_json, validate};

/// The exit status of a command that cannot run (ARC-5.4).
const CANNOT_RUN: u8 = 2;

/// Runs the command line of the process and returns its exit status (ARC-5.4).
pub(crate) fn run() -> ExitCode {
    let arguments: Vec<OsString> = std::env::args_os().skip(1).collect();
    match parse(&arguments) {
        Ok(Command::Help) => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Command::Validate(files)) => match run_validate(&files) {
            Ok(code) => code,
            Err(UsageError(message)) => cannot_run(&message),
        },
        Err(UsageError(message)) => cannot_run(&format!("{message}\n\n{USAGE}")),
    }
}

/// Runs `validate`: writes the report and returns `0` without errors, `1` with errors.
fn run_validate(files: &ValidateFiles) -> Result<ExitCode, UsageError> {
    let model = read(&files.model)?;
    let catalog_schema = read(&files.catalog_schema)?;
    let options = match &files.load_options {
        Some(path) => LoadOptions::from_json(&read(path)?).map_err(|error| {
            UsageError(format!("the load options `{}`: {error}", path.display()))
        })?,
        None => LoadOptions::default(),
    };
    let report = validate(&model, &catalog_schema, &options);
    let text = document_json(&report).map_err(|error| UsageError(error.to_string()))?;
    println!("{text}");
    Ok(if report.errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// The bytes of the file at `path`, unchanged.
fn read(path: &Path) -> Result<Vec<u8>, UsageError> {
    fs::read(path).map_err(|error| UsageError(format!("cannot read `{}`: {error}", path.display())))
}

/// Writes `message` on standard error and returns the exit status `2`.
fn cannot_run(message: &str) -> ExitCode {
    eprintln!("menura: {message}");
    ExitCode::from(CANNOT_RUN)
}

//! The command line of `menura` and of its alias `semantic-compiler` (CON-0.5): it reads its
//! files as bytes, calls `core`, and writes the output document as JSON on standard output
//! (ARC-5.4 to ARC-5.6). Both binaries run this function, so they give the same results.

#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "A2: the command-line wrapper is the only crate allowed to use I/O, the environment and the clock"
)]

use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use crate::arguments::{Command, DiffFiles, ModelFiles, USAGE, UsageError, parse};
use menura_core::{
    CompileErrorDocument, Handle, LoadOptions, dependencies, dependencies_json, diff, diff_json,
    document_json, load, validate,
};

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
        Ok(Command::Validate(files)) => finish(run_validate(&files)),
        Ok(Command::Dependencies(files)) => finish(run_dependencies(&files)),
        Ok(Command::Diff(files)) => finish(run_diff(&files)),
        Err(UsageError(message)) => cannot_run(&format!("{message}\n\n{USAGE}")),
    }
}

/// The exit status of a command that ran, or `2` with its message when it could not.
fn finish(outcome: Result<ExitCode, UsageError>) -> ExitCode {
    outcome.unwrap_or_else(|UsageError(message)| cannot_run(&message))
}

/// The load options of the file at `path`, or the defaults when no file is given (ARC-5.5).
fn load_options(path: Option<&Path>) -> Result<LoadOptions, UsageError> {
    match path {
        Some(path) => LoadOptions::from_json(&read(path)?)
            .map_err(|error| UsageError(format!("the load options `{}`: {error}", path.display()))),
        None => Ok(LoadOptions::default()),
    }
}

/// What loading a model gives the CLI: its handle, or its compile error (ARC-5.6).
enum Loaded {
    Handle(Box<Handle>),
    Error(CompileErrorDocument),
}

/// Loads the model at `model` against the catalog schema at `catalog_schema`.
fn load_model(
    model: &Path,
    catalog_schema: &Path,
    options: &LoadOptions,
) -> Result<Loaded, UsageError> {
    let (model, catalog_schema) = (read(model)?, read(catalog_schema)?);
    Ok(match load(&model, &catalog_schema, None, options) {
        Ok(handle) => Loaded::Handle(Box::new(handle)),
        Err(error) => Loaded::Error(error),
    })
}

/// Writes `error`, a compile error, and returns `1` (ARC-5.4).
fn write_error(error: &CompileErrorDocument) -> Result<ExitCode, UsageError> {
    let text = document_json(error).map_err(|failure| UsageError(failure.to_string()))?;
    println!("{text}");
    Ok(ExitCode::FAILURE)
}

/// Runs `dependencies`: writes the model-dependency document and returns `0`, or the compile
/// error of a model that does not load and returns `1` (ARC-5.6).
fn run_dependencies(files: &ModelFiles) -> Result<ExitCode, UsageError> {
    let options = load_options(files.load_options.as_deref())?;
    let handle = match load_model(&files.model, &files.catalog_schema, &options)? {
        Loaded::Handle(handle) => handle,
        Loaded::Error(error) => return write_error(&error),
    };
    match dependencies(&handle) {
        Ok(document) => {
            let text =
                dependencies_json(&document).map_err(|error| UsageError(error.to_string()))?;
            println!("{text}");
            Ok(ExitCode::SUCCESS)
        }
        Err(error) => write_error(&error),
    }
}

/// Runs `diff`: loads both models with the same load options, old first, and writes the diff
/// of their dependencies; the compile error of the first model that does not load gives `1`
/// (ARC-5.6, DEP-4.4).
fn run_diff(files: &DiffFiles) -> Result<ExitCode, UsageError> {
    let options = load_options(files.load_options.as_deref())?;
    let old = match load_model(&files.old_model, &files.old_catalog_schema, &options)? {
        Loaded::Handle(handle) => handle,
        Loaded::Error(error) => return write_error(&error),
    };
    let new = match load_model(&files.new_model, &files.new_catalog_schema, &options)? {
        Loaded::Handle(handle) => handle,
        Loaded::Error(error) => return write_error(&error),
    };
    let compared =
        dependencies(&old).and_then(|old| dependencies(&new).and_then(|new| diff(&old, &new)));
    match compared {
        Ok(document) => {
            let text = diff_json(&document).map_err(|error| UsageError(error.to_string()))?;
            println!("{text}");
            Ok(ExitCode::SUCCESS)
        }
        Err(error) => write_error(&error),
    }
}

/// Runs `validate`: writes the report and returns `0` without errors, `1` with errors.
fn run_validate(files: &ModelFiles) -> Result<ExitCode, UsageError> {
    let model = read(&files.model)?;
    let catalog_schema = read(&files.catalog_schema)?;
    let options = load_options(files.load_options.as_deref())?;
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

//! The command line of `menura` and of its alias `semantic-compiler` (CON-0.5): it reads its
//! files as bytes, calls `core`, and writes the output document as JSON on standard output
//! (ARC-5.4 to ARC-5.10). Both binaries run this function, so they give the same results.

#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "A2: the command-line wrapper is the only crate allowed to use I/O, the environment and the clock"
)]

mod compile;
mod describe;
mod migrate;
mod model;

use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use crate::arguments::{Command, UsageError, parse};
use crate::files::HandleFiles;
use crate::usage::USAGE;
use menura_core::{
    CompileErrorDocument, Handle, LoadOptions, OutputError, document_json, load, parse_load_options,
};

/// The exit status of a command that cannot run (ARC-5.4).
const CANNOT_RUN: u8 = 2;

/// Runs the command line of the process and returns its exit status (ARC-5.4).
pub(crate) fn run() -> ExitCode {
    let arguments: Vec<OsString> = std::env::args_os().skip(1).collect();
    let outcome = match parse(&arguments) {
        Ok(Command::Help) => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Ok(Command::Validate(files)) => model::run_validate(&files),
        Ok(Command::Compile(files)) => compile::run_compile(&files),
        Ok(Command::Explain(files)) => compile::run_explain(&files),
        Ok(Command::Describe(files)) => describe::run_describe(&files),
        Ok(Command::IntegrityChecks(files)) => describe::run_integrity_checks(&files),
        Ok(Command::Dependencies(files)) => model::run_dependencies(&files),
        Ok(Command::Migrate(files)) => migrate::run_migrate(&files),
        Ok(Command::Diff(files)) => model::run_diff(&files),
        Err(UsageError(message)) => return cannot_run(&format!("{message}\n\n{USAGE}")),
    };
    outcome.unwrap_or_else(|UsageError(message)| cannot_run(&message))
}

/// The load options of the file at `path`, or the defaults when no file is given (ARC-5.5).
fn load_options(path: Option<&Path>) -> Result<LoadOptions, UsageError> {
    match path {
        Some(path) => Ok(parse_load_options(&read(path)?)?),
        None => Ok(LoadOptions::default()),
    }
}

/// What loading a model gives the CLI: its handle, or its compile error (ARC-5.6).
enum Loaded {
    Handle(Box<Handle>),
    Error(CompileErrorDocument),
}

/// Loads the model at `model` against the catalog schema at `catalog_schema`, with the
/// catalog state at `catalog_state` when given.
fn load_model(
    model: &Path,
    catalog_schema: &Path,
    catalog_state: Option<&Path>,
    options: &LoadOptions,
) -> Result<Loaded, UsageError> {
    let (model, catalog_schema) = (read(model)?, read(catalog_schema)?);
    let catalog_state = catalog_state.map(read).transpose()?;
    Ok(
        match load(&model, &catalog_schema, catalog_state.as_deref(), options) {
            Ok(handle) => Loaded::Handle(Box::new(handle)),
            Err(error) => Loaded::Error(error),
        },
    )
}

/// Loads the handle of `files` with their load options (ARC-5.7, ARC-5.8).
fn load_handle(files: &HandleFiles) -> Result<Loaded, UsageError> {
    let options = load_options(files.load_options.as_deref())?;
    load_model(
        &files.model,
        &files.catalog_schema,
        files.catalog_state.as_deref(),
        &options,
    )
}

/// Writes `text`, an output document, on standard output and returns `status` (ARC-5.4).
fn write_document(
    text: Result<String, OutputError>,
    status: ExitCode,
) -> Result<ExitCode, UsageError> {
    let text = text.map_err(|error| UsageError(error.to_string()))?;
    println!("{text}");
    Ok(status)
}

/// Writes `error`, a compile error, and returns `1` (ARC-5.4).
fn write_error(error: &CompileErrorDocument) -> Result<ExitCode, UsageError> {
    write_document(document_json(error), ExitCode::FAILURE)
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

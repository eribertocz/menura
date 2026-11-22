//! The command line of `menura` (ARC-5.5, ARC-5.10), parsed into the command to run. Parsing
//! reads no file: it only decides which command runs, and with which paths. Which arguments a
//! command takes, and the errors of a command line, come from `core`; this module reads the
//! `--flag <value>` syntax and spells each argument as its flag.

use std::ffi::OsString;
use std::path::PathBuf;

use std::collections::BTreeMap;

use menura_core::{Argument, CommandName, Dialect, UsageReason, parse_dialect};

use crate::files::{
    CompileFiles, DescribeFiles, DiffFiles, HandleFiles, IntegrityChecksFiles, MigrateFiles,
    ModelFiles,
};

/// The command a command line asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    /// Write the usage text.
    Help,
    /// `validate` with its files.
    Validate(ModelFiles),
    /// `compile` with its files (ARC-5.7).
    Compile(CompileFiles),
    /// `explain` with its files (ARC-5.7).
    Explain(CompileFiles),
    /// `describe` with its files (ARC-5.8).
    Describe(DescribeFiles),
    /// `integrity-checks` with its files and dialect (ARC-5.8).
    IntegrityChecks(IntegrityChecksFiles),
    /// `dependencies` with its files (ARC-5.6).
    Dependencies(ModelFiles),
    /// `migrate` with its files (ARC-5.9).
    Migrate(MigrateFiles),
    /// `diff` with its files (ARC-5.6).
    Diff(DiffFiles),
}

/// A command line that cannot run, with its message (ARC-5.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageError(pub(crate) String);

impl From<menura_core::UsageError> for UsageError {
    fn from(error: menura_core::UsageError) -> Self {
        Self(error.message(flag))
    }
}

/// The flag of `argument`: `--` and its neutral name, `-` for `_`.
pub(crate) fn flag(argument: Argument) -> String {
    format!("--{}", argument.name().replace('_', "-"))
}

/// The command-line name of `command`: its neutral name, `-` for `_`.
fn command_word(command: CommandName) -> String {
    command.name().replace('_', "-")
}

/// The values of a command line, by argument.
type Given = BTreeMap<Argument, OsString>;

/// Parses the arguments after the program name. `--help` anywhere asks for the usage text.
pub(crate) fn parse(arguments: &[OsString]) -> Result<Command, UsageError> {
    if arguments.iter().any(|argument| argument == "--help") {
        return Ok(Command::Help);
    }
    let Some((word, flags)) = arguments.split_first() else {
        return Err(UsageError("no command given".to_owned()));
    };
    let text = word.to_string_lossy();
    let command = CommandName::ALL
        .into_iter()
        .find(|command| command_word(*command) == text)
        .ok_or_else(|| {
            menura_core::UsageError::new(None, UsageReason::UnknownCommand(text.into_owned()))
        })?;
    let mut given = parse_flags(command, flags)?;
    let present: Vec<Argument> = given.keys().copied().collect();
    command.check_required(&present)?;
    Ok(build(command, &mut given)?)
}

/// The value of each flag of `flags`: each flag at most once, in any order, each followed by
/// its value, and only the flags `command` takes (ARC-5.5).
fn parse_flags(command: CommandName, flags: &[OsString]) -> Result<Given, menura_core::UsageError> {
    let mut given = Given::new();
    let mut remaining = flags.iter();
    while let Some(word) = remaining.next() {
        let text = word.to_string_lossy();
        let argument = text
            .strip_prefix("--")
            .filter(|name| !name.contains('_'))
            .and_then(|name| command.argument(&name.replace('-', "_")).ok())
            .ok_or_else(|| {
                menura_core::UsageError::new(None, UsageReason::UnknownArgument(text.to_string()))
            })?;
        let Some(value) = remaining.next() else {
            return Err(menura_core::UsageError::new(
                Some(argument),
                UsageReason::MissingValue,
            ));
        };
        if given.insert(argument, value.clone()).is_some() {
            return Err(menura_core::UsageError::new(
                Some(argument),
                UsageReason::Repeated,
            ));
        }
    }
    Ok(given)
}

/// The path given for `argument`, taken out of `given`.
fn path(given: &mut Given, argument: Argument) -> Option<PathBuf> {
    given.remove(&argument).map(PathBuf::from)
}

/// The path of the required `argument`; [`CommandName::check_required`] already found it.
fn required(given: &mut Given, argument: Argument) -> Result<PathBuf, menura_core::UsageError> {
    path(given, argument)
        .ok_or_else(|| menura_core::UsageError::new(Some(argument), UsageReason::Missing))
}

/// The command `command` with the values of `given`.
fn build(command: CommandName, given: &mut Given) -> Result<Command, menura_core::UsageError> {
    Ok(match command {
        CommandName::Validate => Command::Validate(model_files(given)?),
        CommandName::Dependencies => Command::Dependencies(model_files(given)?),
        CommandName::Compile => Command::Compile(compile_files(given)?),
        CommandName::Explain => Command::Explain(compile_files(given)?),
        CommandName::Describe => Command::Describe(DescribeFiles {
            handle: handle_files(given)?,
            security_context: required(given, Argument::SecurityContext)?,
        }),
        CommandName::IntegrityChecks => {
            let dialect = match given.remove(&Argument::Dialect) {
                Some(name) => parse_dialect(&name.to_string_lossy())?,
                None => Dialect::Duckdb,
            };
            Command::IntegrityChecks(IntegrityChecksFiles {
                model: model_files(given)?,
                dialect,
            })
        }
        CommandName::Migrate => Command::Migrate(MigrateFiles {
            model: required(given, Argument::Model)?,
            load_options: path(given, Argument::LoadOptions),
        }),
        CommandName::Diff => Command::Diff(DiffFiles {
            old_model: required(given, Argument::OldModel)?,
            old_catalog_schema: required(given, Argument::OldCatalogSchema)?,
            new_model: required(given, Argument::NewModel)?,
            new_catalog_schema: required(given, Argument::NewCatalogSchema)?,
            load_options: path(given, Argument::LoadOptions),
        }),
    })
}

/// The files of `validate`, `dependencies` and `integrity-checks`.
fn model_files(given: &mut Given) -> Result<ModelFiles, menura_core::UsageError> {
    Ok(ModelFiles {
        model: required(given, Argument::Model)?,
        catalog_schema: required(given, Argument::CatalogSchema)?,
        load_options: path(given, Argument::LoadOptions),
    })
}

/// The files that load a handle.
fn handle_files(given: &mut Given) -> Result<HandleFiles, menura_core::UsageError> {
    Ok(HandleFiles {
        model: required(given, Argument::Model)?,
        catalog_schema: required(given, Argument::CatalogSchema)?,
        catalog_state: path(given, Argument::CatalogState),
        load_options: path(given, Argument::LoadOptions),
    })
}

/// The files of `compile` and `explain`; only `compile` takes `--audit-record` (ARC-5.7).
fn compile_files(given: &mut Given) -> Result<CompileFiles, menura_core::UsageError> {
    Ok(CompileFiles {
        handle: handle_files(given)?,
        query: required(given, Argument::Query)?,
        security_context: required(given, Argument::SecurityContext)?,
        compile_options: path(given, Argument::CompileOptions),
        audit_record: path(given, Argument::AuditRecord),
    })
}

#[cfg(test)]
mod tests;

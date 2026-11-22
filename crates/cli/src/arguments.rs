//! The command line of `menura` (ARC-5.5, ARC-5.10), parsed into the command to run. Parsing
//! reads no file: it only decides which command runs, and with which paths.

use std::ffi::OsString;
use std::path::PathBuf;

use menura_core::Dialect;

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

/// A command line that is not the syntax of ARC-5.5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageError(pub(crate) String);

/// Parses the arguments after the program name. `--help` anywhere asks for the usage text.
pub(crate) fn parse(arguments: &[OsString]) -> Result<Command, UsageError> {
    if arguments.iter().any(|argument| argument == "--help") {
        return Ok(Command::Help);
    }
    let Some((command, flags)) = arguments.split_first() else {
        return Err(UsageError("no command given".to_owned()));
    };
    match command.to_str() {
        Some("validate") => parse_model_files(flags).map(Command::Validate),
        Some("compile") => parse_compile_files(flags, true).map(Command::Compile),
        Some("explain") => parse_compile_files(flags, false).map(Command::Explain),
        Some("describe") => parse_describe_files(flags).map(Command::Describe),
        Some("integrity-checks") => {
            parse_integrity_checks_files(flags).map(Command::IntegrityChecks)
        }
        Some("dependencies") => parse_model_files(flags).map(Command::Dependencies),
        Some("migrate") => parse_migrate_files(flags).map(Command::Migrate),
        Some("diff") => parse_diff_files(flags).map(Command::Diff),
        _ => Err(UsageError(format!(
            "unknown command `{}`",
            command.to_string_lossy()
        ))),
    }
}

/// Parses the flags of `validate` and `dependencies` (ARC-5.5, ARC-5.6).
fn parse_model_files(flags: &[OsString]) -> Result<ModelFiles, UsageError> {
    let [model, catalog_schema, load_options] =
        parse_flags(flags, ["--model", "--catalog-schema", "--load-options"])?;
    Ok(ModelFiles {
        model: required(model, "--model")?,
        catalog_schema: required(catalog_schema, "--catalog-schema")?,
        load_options,
    })
}

/// Parses the flags of `compile`, with `--audit-record` when `audit_record` is true, or of
/// `explain`, which refuses it as an unknown flag (ARC-5.7).
fn parse_compile_files(flags: &[OsString], audit_record: bool) -> Result<CompileFiles, UsageError> {
    const COMPILE: [&str; 8] = [
        "--model",
        "--catalog-schema",
        "--query",
        "--security-context",
        "--catalog-state",
        "--load-options",
        "--compile-options",
        "--audit-record",
    ];
    let names = if audit_record {
        &COMPILE[..]
    } else {
        &COMPILE[..COMPILE.len() - 1]
    };
    let mut paths = parse_flag_list(flags, names)?.into_iter();
    let mut next = || paths.next().flatten();
    let (model, catalog_schema, query, security_context) = (next(), next(), next(), next());
    let (catalog_state, load_options, compile_options, audit_record) =
        (next(), next(), next(), next());
    Ok(CompileFiles {
        handle: HandleFiles {
            model: required(model, "--model")?,
            catalog_schema: required(catalog_schema, "--catalog-schema")?,
            catalog_state,
            load_options,
        },
        query: required(query, "--query")?,
        security_context: required(security_context, "--security-context")?,
        compile_options,
        audit_record,
    })
}

/// Parses the flags of `describe` (ARC-5.8).
fn parse_describe_files(flags: &[OsString]) -> Result<DescribeFiles, UsageError> {
    let [
        model,
        catalog_schema,
        security_context,
        catalog_state,
        load_options,
    ] = parse_flags(
        flags,
        [
            "--model",
            "--catalog-schema",
            "--security-context",
            "--catalog-state",
            "--load-options",
        ],
    )?;
    Ok(DescribeFiles {
        handle: HandleFiles {
            model: required(model, "--model")?,
            catalog_schema: required(catalog_schema, "--catalog-schema")?,
            catalog_state,
            load_options,
        },
        security_context: required(security_context, "--security-context")?,
    })
}

/// Parses the flags of `integrity-checks`: `--dialect` is `duckdb`, its default (ARC-5.8).
fn parse_integrity_checks_files(flags: &[OsString]) -> Result<IntegrityChecksFiles, UsageError> {
    let [model, catalog_schema, load_options, dialect] = parse_flags(
        flags,
        ["--model", "--catalog-schema", "--load-options", "--dialect"],
    )?;
    let dialect = match dialect {
        None => Dialect::Duckdb,
        Some(name) if name.as_os_str() == "duckdb" => Dialect::Duckdb,
        Some(name) => {
            return Err(UsageError(format!(
                "unknown dialect `{}`; the dialect is `duckdb`",
                name.display()
            )));
        }
    };
    Ok(IntegrityChecksFiles {
        model: ModelFiles {
            model: required(model, "--model")?,
            catalog_schema: required(catalog_schema, "--catalog-schema")?,
            load_options,
        },
        dialect,
    })
}

/// Parses the flags of `migrate`, which reads no catalog file (ARC-5.9).
fn parse_migrate_files(flags: &[OsString]) -> Result<MigrateFiles, UsageError> {
    let [model, load_options] = parse_flags(flags, ["--model", "--load-options"])?;
    Ok(MigrateFiles {
        model: required(model, "--model")?,
        load_options,
    })
}

/// Parses the flags of `diff` (ARC-5.6).
fn parse_diff_files(flags: &[OsString]) -> Result<DiffFiles, UsageError> {
    let [
        old_model,
        old_catalog_schema,
        new_model,
        new_catalog_schema,
        load_options,
    ] = parse_flags(
        flags,
        [
            "--old-model",
            "--old-catalog-schema",
            "--new-model",
            "--new-catalog-schema",
            "--load-options",
        ],
    )?;
    Ok(DiffFiles {
        old_model: required(old_model, "--old-model")?,
        old_catalog_schema: required(old_catalog_schema, "--old-catalog-schema")?,
        new_model: required(new_model, "--new-model")?,
        new_catalog_schema: required(new_catalog_schema, "--new-catalog-schema")?,
        load_options,
    })
}

/// The path of each flag of `names`, in that order: each flag at most once, in any order, each
/// followed by its path (ARC-5.5).
fn parse_flags<const COUNT: usize>(
    flags: &[OsString],
    names: [&str; COUNT],
) -> Result<[Option<PathBuf>; COUNT], UsageError> {
    let mut paths = parse_flag_list(flags, &names)?.into_iter();
    Ok(std::array::from_fn(|_| paths.next().flatten()))
}

/// The path of each flag of `names`, in that order, as [`parse_flags`] reads them.
fn parse_flag_list(flags: &[OsString], names: &[&str]) -> Result<Vec<Option<PathBuf>>, UsageError> {
    let mut paths: Vec<Option<PathBuf>> = vec![None; names.len()];
    let mut remaining = flags.iter();
    while let Some(flag) = remaining.next() {
        let text = flag.to_string_lossy();
        let Some(index) = names.iter().position(|name| *name == text) else {
            return Err(UsageError(if text.starts_with("--") {
                format!("unknown flag `{text}`")
            } else {
                format!("unexpected argument `{text}`")
            }));
        };
        let Some(path) = remaining.next() else {
            return Err(UsageError(format!("`{text}` needs a path")));
        };
        let slot = paths
            .get_mut(index)
            .ok_or_else(|| UsageError(format!("unknown flag `{text}`")))?;
        if slot.replace(PathBuf::from(path)).is_some() {
            return Err(UsageError(format!("`{text}` is given more than once")));
        }
    }
    Ok(paths)
}

/// The path of the required flag `name`.
fn required(path: Option<PathBuf>, name: &str) -> Result<PathBuf, UsageError> {
    path.ok_or_else(|| UsageError(format!("`{name}` is required")))
}

#[cfg(test)]
mod tests;

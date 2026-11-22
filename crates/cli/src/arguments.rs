//! The command line of `menura` (ARC-5.5), parsed into the command to run. Parsing
//! reads no file: it only decides which command runs, and with which paths.

use std::ffi::OsString;
use std::path::PathBuf;

/// The usage text `--help` writes and every usage error repeats.
pub(crate) const USAGE: &str =
    "usage: menura validate --model <path> --catalog-schema <path> [--load-options <path>]
       menura dependencies --model <path> --catalog-schema <path> [--load-options <path>]
       menura diff --old-model <path> --old-catalog-schema <path> --new-model <path>
                   --new-catalog-schema <path> [--load-options <path>]

`validate` validates an Apache Ossie model against a catalog schema and writes the validation
report. `dependencies` loads the model and writes its model-dependency document. `diff` loads
both models with the same load options, old first, and writes the diff of their dependencies.
Each writes its document as JSON on standard output. Exit status: 0 for a document without
errors, 1 for a report with errors or a compile error, 2 when the command cannot run.

  --model <path>               the model document (YAML or JSON)
  --catalog-schema <path>      the catalog schema document (JSON)
  --old-model <path>           `diff`: the old model document
  --old-catalog-schema <path>  `diff`: the catalog schema of the old model
  --new-model <path>           `diff`: the new model document
  --new-catalog-schema <path>  `diff`: the catalog schema of the new model
  --load-options <path>        a JSON object with load options; an omitted option takes its
                               default
  --help                       writes this text and exits 0
";

/// The command a command line asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    /// Write the usage text.
    Help,
    /// `validate` with its files.
    Validate(ModelFiles),
    /// `dependencies` with its files (ARC-5.6).
    Dependencies(ModelFiles),
    /// `diff` with its files (ARC-5.6).
    Diff(DiffFiles),
}

/// The files of `validate` and `dependencies`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelFiles {
    /// `--model`.
    pub(crate) model: PathBuf,
    /// `--catalog-schema`.
    pub(crate) catalog_schema: PathBuf,
    /// `--load-options`, when given.
    pub(crate) load_options: Option<PathBuf>,
}

/// The files of `diff`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiffFiles {
    /// `--old-model`.
    pub(crate) old_model: PathBuf,
    /// `--old-catalog-schema`.
    pub(crate) old_catalog_schema: PathBuf,
    /// `--new-model`.
    pub(crate) new_model: PathBuf,
    /// `--new-catalog-schema`.
    pub(crate) new_catalog_schema: PathBuf,
    /// `--load-options`, when given.
    pub(crate) load_options: Option<PathBuf>,
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
        Some("dependencies") => parse_model_files(flags).map(Command::Dependencies),
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
    let mut paths: [Option<PathBuf>; COUNT] = std::array::from_fn(|_| None);
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
mod tests {
    use super::*;

    fn arguments(text: &[&str]) -> Vec<OsString> {
        text.iter().map(OsString::from).collect()
    }

    #[test]
    fn arc_5_5_validate_reads_its_three_flags_in_any_order() {
        let command = parse(&arguments(&[
            "validate",
            "--load-options",
            "o.json",
            "--catalog-schema",
            "c.json",
            "--model",
            "m.yaml",
        ]))
        .unwrap();
        assert_eq!(
            command,
            Command::Validate(ModelFiles {
                model: PathBuf::from("m.yaml"),
                catalog_schema: PathBuf::from("c.json"),
                load_options: Some(PathBuf::from("o.json")),
            })
        );
    }

    #[test]
    fn arc_5_5_the_load_options_flag_is_optional() {
        let Command::Validate(files) = parse(&arguments(&[
            "validate",
            "--model",
            "m.yaml",
            "--catalog-schema",
            "c.json",
        ]))
        .unwrap() else {
            panic!("validate");
        };
        assert_eq!(files.load_options, None);
    }

    #[test]
    fn arc_5_6_diff_reads_its_five_flags_in_any_order() {
        let command = parse(&arguments(&[
            "diff",
            "--new-catalog-schema",
            "nc.json",
            "--old-model",
            "o.yaml",
            "--new-model",
            "n.yaml",
            "--old-catalog-schema",
            "oc.json",
        ]))
        .unwrap();
        assert_eq!(
            command,
            Command::Diff(DiffFiles {
                old_model: PathBuf::from("o.yaml"),
                old_catalog_schema: PathBuf::from("oc.json"),
                new_model: PathBuf::from("n.yaml"),
                new_catalog_schema: PathBuf::from("nc.json"),
                load_options: None,
            })
        );
    }

    #[test]
    fn arc_5_6_dependencies_takes_the_flags_of_validate() {
        let command = parse(&arguments(&[
            "dependencies",
            "--model",
            "m.yaml",
            "--catalog-schema",
            "c.json",
        ]))
        .unwrap();
        assert!(matches!(command, Command::Dependencies(_)));
        assert!(parse(&arguments(&["dependencies", "--old-model", "m"])).is_err());
    }

    #[test]
    fn arc_5_5_help_wins_over_every_other_argument() {
        assert_eq!(
            parse(&arguments(&["validate", "--x", "--help"])),
            Ok(Command::Help)
        );
    }

    #[test]
    fn arc_5_5_a_command_line_outside_the_syntax_is_a_usage_error() {
        for line in [
            &[][..],
            &["describe"][..],
            &["validate", "--model", "m.yaml"][..],
            &[
                "validate",
                "--model",
                "m",
                "--model",
                "m",
                "--catalog-schema",
                "c",
            ][..],
            &[
                "validate",
                "--model",
                "m",
                "--catalog-schema",
                "c",
                "--dialect",
                "d",
            ][..],
            &["validate", "--model", "m", "--catalog-schema", "c", "x"][..],
            &["validate", "--model", "m", "--catalog-schema"][..],
            &["validate", "--model=m", "--catalog-schema", "c"][..],
        ] {
            assert!(parse(&arguments(line)).is_err(), "{line:?}");
        }
    }
}

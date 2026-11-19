//! The command line of `semantic-compiler` (ARC-5.5), parsed into the command to run. Parsing
//! reads no file: it only decides which command runs, and with which paths.

use std::ffi::OsString;
use std::path::PathBuf;

/// The usage text `--help` writes and every usage error repeats.
pub(crate) const USAGE: &str = "usage: semantic-compiler validate --model <path> --catalog-schema <path> [--load-options <path>]

Validates an Apache Ossie model against a catalog schema and writes the validation report as
JSON on standard output. Exit status: 0 when the report has no error, 1 when it has errors,
2 when the command cannot run.

  --model <path>           the model document (YAML or JSON)
  --catalog-schema <path>  the catalog schema document (JSON)
  --load-options <path>    a JSON object with load options; an omitted option takes its default
  --help                   writes this text and exits 0
";

/// The command a command line asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    /// Write the usage text.
    Help,
    /// `validate` with its files.
    Validate(ValidateFiles),
}

/// The files of `validate`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidateFiles {
    /// `--model`.
    pub(crate) model: PathBuf,
    /// `--catalog-schema`.
    pub(crate) catalog_schema: PathBuf,
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
    if command != "validate" {
        return Err(UsageError(format!(
            "unknown command `{}`",
            command.to_string_lossy()
        )));
    }
    parse_validate(flags).map(Command::Validate)
}

/// Parses the flags of `validate`: each at most once, in any order, each followed by its path.
fn parse_validate(flags: &[OsString]) -> Result<ValidateFiles, UsageError> {
    let (mut model, mut catalog_schema, mut load_options) = (None, None, None);
    let mut remaining = flags.iter();
    while let Some(flag) = remaining.next() {
        let slot = match flag.to_str() {
            Some("--model") => &mut model,
            Some("--catalog-schema") => &mut catalog_schema,
            Some("--load-options") => &mut load_options,
            Some(text) if text.starts_with("--") => {
                return Err(UsageError(format!("unknown flag `{text}`")));
            }
            _ => {
                return Err(UsageError(format!(
                    "unexpected argument `{}`",
                    flag.to_string_lossy()
                )));
            }
        };
        let name = flag.to_string_lossy();
        let Some(path) = remaining.next() else {
            return Err(UsageError(format!("`{name}` needs a path")));
        };
        if slot.replace(PathBuf::from(path)).is_some() {
            return Err(UsageError(format!("`{name}` is given more than once")));
        }
    }
    Ok(ValidateFiles {
        model: model.ok_or_else(|| UsageError("`--model` is required".to_owned()))?,
        catalog_schema: catalog_schema
            .ok_or_else(|| UsageError("`--catalog-schema` is required".to_owned()))?,
        load_options,
    })
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
            Command::Validate(ValidateFiles {
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

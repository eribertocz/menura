//! The usage error: a call that a command cannot run, with the argument it is about and why.

use std::fmt;

use super::argument::Argument;

/// A call that does not follow the syntax of its command, or whose options object is not one
/// (ARC-5.5, ARC-5.7, ARC-5.8). The CLI exits `2` with its message; the MCP server answers
/// `-32602` (ARC-6.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageError {
    /// The argument the error is about, when there is one.
    pub argument: Option<Argument>,
    /// Why the call cannot run.
    pub reason: UsageReason,
}

/// Why a call cannot run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsageReason {
    /// No command has this name.
    UnknownCommand(String),
    /// The command takes no argument of this name, as the caller wrote it.
    UnknownArgument(String),
    /// A required argument is missing.
    Missing,
    /// The argument is given more than once.
    Repeated,
    /// The argument has no value.
    MissingValue,
    /// The options object is not one of the options of section 3.5: not UTF-8 JSON, not an
    /// object, an unknown option, or a value of the wrong type; the text says which.
    InvalidOptions(String),
    /// The dialect is not one the release generates (ARC-5.8).
    UnknownDialect(String),
    /// `audit: true` without `audit_record` (ARC-5.7).
    AuditWithoutRecord,
    /// `audit_record` without `audit: true` (ARC-5.7).
    RecordWithoutAudit,
}

impl UsageError {
    /// The error `reason` about `argument`.
    #[must_use]
    pub fn new(argument: Option<Argument>, reason: UsageReason) -> Self {
        Self { argument, reason }
    }

    /// The message of the error, with every argument written as `spell` writes it: `--model`
    /// for the CLI, `model` for an MCP member.
    ///
    /// ```
    /// use menura_core::{Argument, UsageError, UsageReason};
    ///
    /// let error = UsageError::new(Some(Argument::CatalogSchema), UsageReason::Missing);
    /// let flag = |argument: Argument| format!("--{}", argument.name().replace('_', "-"));
    /// assert_eq!(error.message(flag), "`--catalog-schema` is required");
    /// assert_eq!(error.to_string(), "`catalog_schema` is required");
    /// ```
    pub fn message(&self, spell: impl Fn(Argument) -> String) -> String {
        let name = self.argument.map_or_else(
            || "the argument".to_owned(),
            |argument| format!("`{}`", spell(argument)),
        );
        match &self.reason {
            UsageReason::UnknownCommand(command) => format!("unknown command `{command}`"),
            UsageReason::UnknownArgument(argument) => format!("unknown argument `{argument}`"),
            UsageReason::Missing => format!("{name} is required"),
            UsageReason::Repeated => format!("{name} is given more than once"),
            UsageReason::MissingValue => format!("{name} needs a value"),
            UsageReason::InvalidOptions(detail) => format!("{name}: {detail}"),
            UsageReason::UnknownDialect(dialect) => {
                format!("unknown dialect `{dialect}`; the dialect is `duckdb`")
            }
            UsageReason::AuditWithoutRecord => format!(
                "the compile option `audit: true` needs `{}`",
                spell(Argument::AuditRecord)
            ),
            UsageReason::RecordWithoutAudit => format!(
                "`{}` needs the compile option `audit: true`",
                spell(Argument::AuditRecord)
            ),
        }
    }
}

impl fmt::Display for UsageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message(|argument| argument.name().to_owned()))
    }
}

impl std::error::Error for UsageError {}

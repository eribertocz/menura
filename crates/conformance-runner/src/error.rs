//! Typed errors of the conformance runner, each with the rule it enforces and the file involved.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// A failure of the runner, carrying the rule of `spec/08-conformance-suite.md` it enforces and
/// the file or folder where it happened.
#[derive(Debug)]
pub struct RunnerError {
    /// What went wrong.
    pub kind: RunnerErrorKind,
    /// The file or folder involved.
    pub path: PathBuf,
}

/// The kinds of runner failure.
#[derive(Debug)]
pub enum RunnerErrorKind {
    /// A file or folder of the layout cannot be read (CS-1).
    Unreadable(io::Error),
    /// The requested area has no folder under `cases/` (CS-4.1).
    UnknownArea(String),
    /// A case or fixture file is not valid YAML or JSON of the expected shape (CS-1, CS-2).
    Malformed(String),
    /// The case file name differs from its `id` (CS-1).
    IdMismatch(String),
    /// The case lists no rule ID (CS-2.1).
    NoRules,
    /// An `outcome: plan` case has no `rows` (CS-2.2).
    PlanWithoutRows,
    /// The outcome requires a security context and the case names none (CS-2).
    MissingSecurityContext,
    /// The `DuckDB` database cannot be prepared (CS-4.2).
    Database(duckdb::Error),
}

impl RunnerErrorKind {
    /// The rule of `spec/08-conformance-suite.md` this failure enforces.
    #[must_use]
    pub fn rule(&self) -> &'static str {
        match self {
            Self::Unreadable(_) | Self::Malformed(_) | Self::IdMismatch(_) => "CS-1",
            Self::UnknownArea(_) => "CS-4.1",
            Self::NoRules => "CS-2.1",
            Self::PlanWithoutRows => "CS-2.2",
            Self::MissingSecurityContext => "CS-2",
            Self::Database(_) => "CS-4.2",
        }
    }
}

impl RunnerError {
    /// Builds an error of the given kind for a path.
    #[must_use]
    pub fn new(kind: RunnerErrorKind, path: impl Into<PathBuf>) -> Self {
        Self {
            kind,
            path: path.into(),
        }
    }
}

impl fmt::Display for RunnerErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable(error) => write!(formatter, "cannot be read: {error}"),
            Self::UnknownArea(area) => write!(formatter, "area `{area}` has no cases folder"),
            Self::Malformed(message) => write!(formatter, "malformed: {message}"),
            Self::IdMismatch(id) => write!(formatter, "file name differs from case id `{id}`"),
            Self::NoRules => formatter.write_str("`rules` lists no rule ID"),
            Self::PlanWithoutRows => formatter.write_str("`outcome: plan` requires `rows`"),
            Self::MissingSecurityContext => {
                formatter.write_str("this outcome requires `security_context`")
            }
            Self::Database(error) => write!(formatter, "database preparation failed: {error}"),
        }
    }
}

impl fmt::Display for RunnerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: {}: {}",
            self.kind.rule(),
            self.path.display(),
            self.kind
        )
    }
}

impl std::error::Error for RunnerError {}

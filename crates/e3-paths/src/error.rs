//! The errors of stage E3 (E3-6), each with its specification code and its location in the
//! query.

use std::fmt;

use menura_contracts::types::compiled_plan::{Diagnostic, DiagnosticLocation, InputDocument};
use menura_representations::origin::QueryLocation;

use crate::violation::InvariantViolation;

/// The specification code of an E3 error (E3-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PathCode {
    /// `E_NO_PATH`: a dimension dataset that no metric reaches (E3-3.5, E2-5 row 13).
    NoPath,
    /// `E_PARTIAL_DIMENSION_REACHABILITY`: a dimension dataset that only some metrics reach
    /// (E3-3.5, E2-5 row 14).
    PartialDimensionReachability,
    /// `E_NO_ANCHOR`: a dimension-only query without an anchor (E3-5.1, E2-5 row 15).
    NoAnchor,
    /// `E_UNSUPPORTED_FILTER_PATH`: a filter dataset that some metric, or an anchor, cannot
    /// reach (E3-3.6, E3-5.2, E2-5 row 16).
    UnsupportedFilterPath,
    /// `E_AMBIGUOUS_PATH`: two or more paths to a needed dataset and no `paths` choice
    /// (E3-3.2), or a dimension-only query with more than one anchor (E3-5.1); E2-5 row 17.
    AmbiguousPath,
    /// `E3-01`: a `paths` entry that no branch uses (E3-3.3, E2-5 row 17, after
    /// `E_AMBIGUOUS_PATH`).
    UnusedPathEntry,
}

impl PathCode {
    /// The code as the specification writes it, such as `E_NO_PATH`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoPath => "E_NO_PATH",
            Self::PartialDimensionReachability => "E_PARTIAL_DIMENSION_REACHABILITY",
            Self::NoAnchor => "E_NO_ANCHOR",
            Self::UnsupportedFilterPath => "E_UNSUPPORTED_FILTER_PATH",
            Self::AmbiguousPath => "E_AMBIGUOUS_PATH",
            Self::UnusedPathEntry => "E3-01",
        }
    }
}

impl fmt::Display for PathCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// An E3 error of the query: its code, its message and where it is located.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathDiagnostic {
    /// The specification code.
    pub code: PathCode,
    /// What is wrong and how to fix it.
    pub message: String,
    /// The query element the error is located at.
    pub location: QueryLocation,
}

impl PathDiagnostic {
    /// The error `code` with `message`, located at `location`.
    pub(crate) fn new(code: PathCode, message: String, location: &QueryLocation) -> Self {
        Self {
            code,
            message,
            location: location.clone(),
        }
    }

    /// The diagnostic of the compile error document (PLN-5), located in the query.
    #[must_use]
    pub fn to_diagnostic(&self) -> Diagnostic {
        let position = self.location.position();
        Diagnostic {
            code: self.code.as_str().to_owned(),
            message: self.message.clone(),
            location: Some(DiagnosticLocation {
                document: InputDocument::Query,
                pointer: self.location.pointer().to_owned(),
                line: position.and_then(|position| u64::try_from(position.line()).ok()),
                column: position.and_then(|position| u64::try_from(position.column()).ok()),
            }),
        }
    }
}

/// Why stage E3 stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathError {
    /// An error of the query, with its specification code (E3-6).
    Diagnostic(PathDiagnostic),
    /// A bound query that E2 cannot produce: a defect of the compiler, without code.
    Invariant(InvariantViolation),
}

impl fmt::Display for PathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Diagnostic(diagnostic) => write!(
                formatter,
                "{} at {}: {}",
                diagnostic.code,
                diagnostic.location.pointer(),
                diagnostic.message
            ),
            Self::Invariant(violation) => violation.fmt(formatter),
        }
    }
}

impl std::error::Error for PathError {}

impl From<PathDiagnostic> for PathError {
    fn from(diagnostic: PathDiagnostic) -> Self {
        Self::Diagnostic(diagnostic)
    }
}

impl From<InvariantViolation> for PathError {
    fn from(violation: InvariantViolation) -> Self {
        Self::Invariant(violation)
    }
}

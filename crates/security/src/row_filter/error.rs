//! Why the row filter of a dataset cannot be built.

use std::fmt;

use menura_contracts::types::compiled_plan::{Diagnostic, DiagnosticLocation, InputDocument};
use menura_model::internal_model::SourceLocation;

/// Why [`super::row_filter`] failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowFilterError {
    /// `ESEC-02`: an applicable policy reads an attribute the security context does not
    /// provide (SEC-3.4, E4-6.9).
    MissingAttribute {
        /// The name of the attribute, as declared.
        attribute: String,
    },
    /// A policy whose condition E0 did not parse. A loaded model never holds one, so this is
    /// an internal error of the compiler, with no specification code.
    UnparsedCondition {
        /// Where the policy is declared.
        location: SourceLocation,
    },
}

impl RowFilterError {
    /// The `ESEC-02` diagnostic of a missing attribute, located at `/attributes` of the
    /// security context, whose message names only the attribute (E4-6.9, PLN-5.2, SCY-4a.1);
    /// `None` for an internal error.
    #[must_use]
    pub fn to_diagnostic(&self) -> Option<Diagnostic> {
        match self {
            Self::MissingAttribute { attribute } => Some(Diagnostic {
                code: "ESEC-02".to_owned(),
                message: format!(
                    "the security context does not provide the attribute `{attribute}`; add it \
                     to `attributes`"
                ),
                location: Some(DiagnosticLocation {
                    document: InputDocument::SecurityContext,
                    pointer: "/attributes".to_owned(),
                    line: None,
                    column: None,
                }),
            }),
            Self::UnparsedCondition { .. } => None,
        }
    }
}

impl fmt::Display for RowFilterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAttribute { attribute } => {
                write!(formatter, "ESEC-02: missing attribute `{attribute}`")
            }
            Self::UnparsedCondition { location } => write!(
                formatter,
                "a row policy at {} has no parsed condition",
                location.pointer
            ),
        }
    }
}

impl std::error::Error for RowFilterError {}

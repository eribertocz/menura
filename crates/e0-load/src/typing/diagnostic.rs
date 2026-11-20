//! The diagnostics of typing (E0F-8), as data types only.

use std::fmt;

use menura_expressions::typing::TypingCode;

use crate::pointer::JsonPointer;
use crate::read::Location;

/// The load warning reported for every typing problem (E0-4 table).
pub(crate) const TYPING_WARNING: &str = "W0F-01";

/// One typing diagnostic: its code, the node it is about and a description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypingDiagnostic {
    /// The specification code; the query error of the element it makes unusable.
    pub code: TypingCode,
    /// The JSON Pointer of the offending node: the `expression` of a field or metric, or the
    /// `from_columns` entry of a relationship key pair.
    pub pointer: JsonPointer,
    /// Where that node starts in the document text.
    pub location: Location,
    /// What the problem is, in English, for diagnostics.
    pub message: String,
}

impl fmt::Display for TypingDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} ({}) at `{}` (line {}, column {}): {}",
            TYPING_WARNING,
            self.code,
            self.pointer,
            self.location.line,
            self.location.column,
            self.message
        )
    }
}

impl std::error::Error for TypingDiagnostic {}

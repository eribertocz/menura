//! The diagnostics of reference resolution (E0D-4), as data types only.

use std::fmt;

use crate::pointer::JsonPointer;
use crate::read::Location;

/// The code of a reference diagnostic (E0D-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReferenceCode {
    /// `E0D-01`: a duplicate name, or a name longer than 128 characters.
    InvalidName,
    /// `E0D-02`: a dataset `source` that the catalog schema does not describe.
    SourceNotInCatalog,
    /// `E0D-03`: a field expression reads a column that its catalog relation does not have.
    UnknownColumn,
    /// `E0D-04`: a metric refers to a missing dataset or field.
    MissingMetricReference,
    /// `E0D-05`: a relationship refers to a missing dataset.
    MissingRelationshipDataset,
    /// `E0D-06`: a key or relationship column names neither a field nor a source column.
    UnresolvedKeyName,
    /// `E0D-07`: a key name is both a field and a different source column.
    KeyFieldIsOtherColumn,
}

impl ReferenceCode {
    /// The load warning reported for every missing reference (E0-4 table).
    pub const MISSING_REFERENCE_WARNING: &'static str = "W0D-01";

    /// The code as the specification writes it, such as `E0D-04`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidName => "E0D-01",
            Self::SourceNotInCatalog => "E0D-02",
            Self::UnknownColumn => "E0D-03",
            Self::MissingMetricReference => "E0D-04",
            Self::MissingRelationshipDataset => "E0D-05",
            Self::UnresolvedKeyName => "E0D-06",
            Self::KeyFieldIsOtherColumn => "E0D-07",
        }
    }

    /// Whether the code rejects the model (E0-4 table: `E0D-01`, DEV-2, and `E0D-02`, DEV-3).
    /// Every other code makes its element unusable.
    #[must_use]
    pub fn rejects(self) -> bool {
        matches!(self, Self::InvalidName | Self::SourceNotInCatalog)
    }

    /// The code the load reports: the code itself when it rejects, otherwise
    /// [`Self::MISSING_REFERENCE_WARNING`]; the code itself is then the query error of the
    /// unusable element (E0-4.1).
    #[must_use]
    pub fn load_code(self) -> &'static str {
        if self.rejects() {
            self.as_str()
        } else {
            Self::MISSING_REFERENCE_WARNING
        }
    }
}

impl fmt::Display for ReferenceCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One reference diagnostic: its code, the node it is about and a description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceDiagnostic {
    /// The specification code.
    pub code: ReferenceCode,
    /// The JSON Pointer of the offending node (E0B-3.6).
    pub pointer: JsonPointer,
    /// Where that node starts in the document text.
    pub location: Location,
    /// What the problem is, in English, for diagnostics.
    pub message: String,
}

impl fmt::Display for ReferenceDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} ({}) at `{}` (line {}, column {}): {}",
            self.code.load_code(),
            self.code,
            self.pointer,
            self.location.line,
            self.location.column,
            self.message
        )
    }
}

impl std::error::Error for ReferenceDiagnostic {}

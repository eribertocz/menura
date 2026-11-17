//! The diagnostics of relationship analysis (E0E-7), as data types only.

use std::fmt;

use crate::pointer::JsonPointer;
use crate::read::Location;

/// The code of a relationship diagnostic (E0E-7).
///
/// The order of the variants is the order in which diagnostics at the same node are reported
/// (E0-4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum RelationshipCode {
    /// `E0E-01`: `from_columns` and `to_columns` differ in length (E0E-3.1).
    ColumnCountMismatch,
    /// `E0E-02`: the relationship goes to or from a summary dataset (E0E-3.6).
    SummaryEnd,
    /// `W0E-01`: more than one path between two datasets (E0E-4.2).
    SeveralPaths,
    /// `W0E-02`: `to_columns` is not a declared key of `to` (E0E-3.2).
    UndeclaredKey,
    /// `W0E-03`: a self-relationship, not used in paths (E0E-3.5).
    SelfRelationship,
}

impl RelationshipCode {
    /// The load warning reported for every invalid relationship, which is excluded from the
    /// graph (E0-4 table, E0E-7).
    pub(crate) const INVALID_RELATIONSHIP_WARNING: &'static str = "W0E-04";

    /// The query error of a query that names an invalid relationship (E0-4 table).
    pub(crate) const INVALID_RELATIONSHIP_QUERY_ERROR: &'static str = "E_NAME_NOT_FOUND";

    /// The code as the specification writes it, such as `E0E-01`.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ColumnCountMismatch => "E0E-01",
            Self::SummaryEnd => "E0E-02",
            Self::SeveralPaths => "W0E-01",
            Self::UndeclaredKey => "W0E-02",
            Self::SelfRelationship => "W0E-03",
        }
    }

    /// Whether the code makes its relationship unusable (E0-4 table: `E0E-01`, `E0E-02`).
    pub(crate) fn invalidates(self) -> bool {
        matches!(self, Self::ColumnCountMismatch | Self::SummaryEnd)
    }

    /// The code the load reports: [`Self::INVALID_RELATIONSHIP_WARNING`] for a code that makes
    /// its relationship unusable, otherwise the warning itself (E0-4.1).
    pub(crate) fn load_code(self) -> &'static str {
        if self.invalidates() {
            Self::INVALID_RELATIONSHIP_WARNING
        } else {
            self.as_str()
        }
    }
}

impl fmt::Display for RelationshipCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One relationship diagnostic: its code, the node it is about and a description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RelationshipDiagnostic {
    /// The specification code.
    pub(crate) code: RelationshipCode,
    /// The JSON Pointer of the node it is about: the relationship, or the member of it at fault;
    /// for `W0E-01`, the dataset the paths start from.
    pub(crate) pointer: JsonPointer,
    /// Where that node starts in the document text.
    pub(crate) location: Location,
    /// What the problem is, in English, for diagnostics.
    pub(crate) message: String,
}

impl fmt::Display for RelationshipDiagnostic {
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

impl std::error::Error for RelationshipDiagnostic {}

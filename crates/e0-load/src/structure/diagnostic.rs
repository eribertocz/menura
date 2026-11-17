//! The diagnostics of structure validation (E0B-4), as data types only.

use std::fmt;

use crate::pointer::JsonPointer;
use crate::read::Location;

/// The code of a structure diagnostic (E0B-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StructureCode {
    /// `E0B-01`: `version` is missing or not `"0.1.1"`.
    Version,
    /// `E0B-02`: a required property is missing.
    MissingProperty,
    /// `E0B-03`: a value has the wrong type.
    WrongType,
    /// `E0B-04`: a property is not allowed.
    PropertyNotAllowed,
    /// `E0B-05`: a value is outside an enumeration.
    OutsideEnumeration,
    /// `E0B-06`: a list that needs at least one item is empty.
    EmptyList,
    /// `E0B-07`: several semantic models and no `model_name`, or `model_name` names none of them.
    ModelSelection,
    /// `W0B-01`: a variant in a dialect other than `ANSI_SQL`, kept and not compiled.
    OtherDialect,
    /// `W0B-02`: an expression without a single `ANSI_SQL` variant; its element is unusable.
    NoSingleAnsiSql,
    /// `W0B-03`: a development-line `datatype` without a logical kind (OSS-5.2); its element is
    /// unusable.
    UnknownDatatype,
}

impl StructureCode {
    /// The code as the specification writes it, such as `E0B-04`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Version => "E0B-01",
            Self::MissingProperty => "E0B-02",
            Self::WrongType => "E0B-03",
            Self::PropertyNotAllowed => "E0B-04",
            Self::OutsideEnumeration => "E0B-05",
            Self::EmptyList => "E0B-06",
            Self::ModelSelection => "E0B-07",
            Self::OtherDialect => "W0B-01",
            Self::NoSingleAnsiSql => "W0B-02",
            Self::UnknownDatatype => "W0B-03",
        }
    }

    /// Whether the code is a load warning; every other code rejects the model (E0-4.1).
    #[must_use]
    pub fn is_warning(self) -> bool {
        matches!(
            self,
            Self::OtherDialect | Self::NoSingleAnsiSql | Self::UnknownDatatype
        )
    }
}

impl fmt::Display for StructureCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One structure diagnostic: its code, the node it is about and a description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureDiagnostic {
    /// The specification code.
    pub code: StructureCode,
    /// The JSON Pointer of the node (E0B-3.6). A missing property is located at the object that
    /// lacks it.
    pub pointer: JsonPointer,
    /// Where that node starts in the document text.
    pub location: Location,
    /// What the problem is, in English, for diagnostics.
    pub message: String,
}

impl fmt::Display for StructureDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at `{}` (line {}, column {}): {}",
            self.code, self.pointer, self.location.line, self.location.column, self.message
        )
    }
}

impl std::error::Error for StructureDiagnostic {}

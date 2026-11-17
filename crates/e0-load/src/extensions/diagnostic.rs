//! The diagnostics of the common extension rules (E0C-3), as data types only.

use std::fmt;

use crate::pointer::JsonPointer;
use crate::read::Location;

/// The code of a common extension diagnostic (E0C-3).
///
/// `E0C-03`, a wrong value type of an extension key, is raised by the validation of each
/// extension (E0C-2.5), not by the common rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExtensionCode {
    /// `E0C-01`: `data.version` is missing or not supported.
    Version,
    /// `E0C-02`: a key of `data` that no extension defines.
    UnknownKey,
    /// `E0C-04`: more than one entry of ours on the same object.
    SeveralEntries,
    /// `E0C-05`: an extension key on an object type it is not valid on.
    WrongObject,
    /// `E0C-06`: a member name repeated in an object inside `data` of an entry of ours.
    RepeatedMember,
    /// `W0C-01`: a `COMMON` entry whose `data` is not valid JSON; kept, not interpreted.
    NotJson,
}

impl ExtensionCode {
    /// The code as the specification writes it, such as `E0C-04`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Version => "E0C-01",
            Self::UnknownKey => "E0C-02",
            Self::SeveralEntries => "E0C-04",
            Self::WrongObject => "E0C-05",
            Self::RepeatedMember => "E0C-06",
            Self::NotJson => "W0C-01",
        }
    }

    /// Whether the code is a load warning; every other code rejects the model (E0-4.1).
    #[must_use]
    pub fn is_warning(self) -> bool {
        matches!(self, Self::NotJson)
    }
}

impl fmt::Display for ExtensionCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One common extension diagnostic: its code, the node it is about and a description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionDiagnostic {
    /// The specification code.
    pub code: ExtensionCode,
    /// The JSON Pointer of the node (E0B-3.6): the `data` member of the entry, or the entry
    /// itself for `E0C-04`. A problem inside `data` cannot be located deeper, because `data` is a
    /// JSON text.
    pub pointer: JsonPointer,
    /// Where that node starts in the document text.
    pub location: Location,
    /// What the problem is, in English, for diagnostics.
    pub message: String,
}

impl fmt::Display for ExtensionDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at `{}` (line {}, column {}): {}",
            self.code, self.pointer, self.location.line, self.location.column, self.message
        )
    }
}

impl std::error::Error for ExtensionDiagnostic {}

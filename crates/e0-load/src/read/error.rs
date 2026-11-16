//! The errors of reading (E0A-3).

use std::error::Error;
use std::fmt;

use super::tree::Location;

/// The code of a reading error (E0A-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReadErrorCode {
    /// `E0A-01`: the document is not valid UTF-8.
    InvalidEncoding,
    /// `E0A-02`: the YAML cannot be parsed.
    Unparsable,
    /// `E0A-03`: the stream holds more than one YAML document.
    SeveralDocuments,
    /// `E0A-04`: a mapping repeats a key.
    DuplicateKey,
    /// `E0A-05`: a tag or a value cannot be constructed.
    Unconstructible,
}

impl ReadErrorCode {
    /// The code as the specification writes it, such as `E0A-04`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidEncoding => "E0A-01",
            Self::Unparsable => "E0A-02",
            Self::SeveralDocuments => "E0A-03",
            Self::DuplicateKey => "E0A-04",
            Self::Unconstructible => "E0A-05",
        }
    }
}

impl fmt::Display for ReadErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A document that cannot be read: its code, where the problem is, and a description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadError {
    /// The specification code.
    pub code: ReadErrorCode,
    /// Where the problem is in the document text.
    pub location: Location,
    /// What the problem is, in English, for diagnostics.
    pub message: String,
}

impl ReadError {
    /// Builds an error.
    pub(crate) fn new(code: ReadErrorCode, location: Location, message: impl Into<String>) -> Self {
        Self {
            code,
            location,
            message: message.into(),
        }
    }

    /// Builds an `E0A-05` error.
    pub(crate) fn unconstructible(location: Location, message: impl Into<String>) -> Self {
        Self::new(ReadErrorCode::Unconstructible, location, message)
    }
}

impl fmt::Display for ReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at line {}, column {}: {}",
            self.code, self.location.line, self.location.column, self.message
        )
    }
}

impl Error for ReadError {}

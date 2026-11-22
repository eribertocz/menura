//! The error and the warning of E1, with their specification codes (E1-6, E1-2.4).

use std::fmt;

use menura_contracts::AdmissionError;
use menura_contracts::types::compiled_plan::{Diagnostic, DiagnosticLocation, InputDocument};

/// The specification code of an E1 error (E1-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseErrorCode {
    /// `E_INVALID_QUERY`: the query is not JSON or fails its JSON Schema.
    InvalidQuery,
    /// `EQRY-08`: unsupported `contract` or `contract_version`.
    Eqry08,
    /// `EQRY-01`: both or neither of `query` and `sql`.
    Eqry01,
    /// `EQRY-02`: unknown time zone (E1-2.3).
    Eqry02,
    /// `EQRY-05`: a SQL text that does not parse, or a construct outside the version 1
    /// subset (E1-4).
    Eqry05,
    /// `EQRY-06`: unsupported extension `version`.
    Eqry06,
    /// `EQRY-13`: a SQL `SELECT` item that is not a plain field reference and has no alias
    /// (E1-4).
    Eqry13,
    /// `E0F-01`: a filter text does not parse (E1-3.2).
    E0f01,
    /// `E0F-02`: a filter text uses a construct outside the subset (E1-3.2).
    E0f02,
    /// `E_NOT_YET_SUPPORTED`: before 0.5.0, the query uses the SQL form (VS-2).
    NotYetSupported,
    /// `E_EMPTY_AGGREGATION_QUERY`: no dimensions and no metrics (QRY-3.1).
    EmptyAggregationQuery,
}

impl ParseErrorCode {
    /// The code as the specification writes it.
    ///
    /// ```
    /// use menura_e1_parse::ParseErrorCode;
    /// assert_eq!(ParseErrorCode::Eqry02.as_str(), "EQRY-02");
    /// ```
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidQuery => "E_INVALID_QUERY",
            Self::Eqry08 => "EQRY-08",
            Self::Eqry01 => "EQRY-01",
            Self::Eqry02 => "EQRY-02",
            Self::Eqry05 => "EQRY-05",
            Self::Eqry06 => "EQRY-06",
            Self::Eqry13 => "EQRY-13",
            Self::E0f01 => "E0F-01",
            Self::E0f02 => "E0F-02",
            Self::NotYetSupported => "E_NOT_YET_SUPPORTED",
            Self::EmptyAggregationQuery => "E_EMPTY_AGGREGATION_QUERY",
        }
    }
}

impl fmt::Display for ParseErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// The first error of a query that E1 rejects, located in the query document (E1-2.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Specification code.
    pub code: ParseErrorCode,
    /// JSON Pointer (RFC 6901) of the offending value.
    pub pointer: String,
    /// Line and column (from 1) of a JSON syntax error.
    pub line_column: Option<(u64, u64)>,
    /// English description naming the offending item.
    pub message: String,
    /// The `WQRY-01` warnings of a query that passed the envelope and the JSON Schema before
    /// this error (E1-2.4), in document order; empty for an earlier error.
    pub warnings: Vec<ParseWarning>,
}

impl ParseError {
    /// An error of `code` at `pointer`, without a line and column or warnings.
    pub(crate) fn new(code: ParseErrorCode, pointer: &str, message: String) -> Self {
        Self {
            code,
            pointer: pointer.to_owned(),
            line_column: None,
            message,
            warnings: Vec::new(),
        }
    }

    /// The same error with the `WQRY-01` `warnings` gathered before it.
    pub(crate) fn with_warnings(self, warnings: Vec<ParseWarning>) -> Self {
        Self { warnings, ..self }
    }

    /// The diagnostic of the compile error document (PLN-5), located in the query.
    #[must_use]
    pub fn diagnostic(&self) -> Diagnostic {
        let (line, column) = self
            .line_column
            .map_or((None, None), |(line, column)| (Some(line), Some(column)));
        Diagnostic {
            code: self.code.as_str().to_owned(),
            message: self.message.clone(),
            location: Some(DiagnosticLocation {
                document: InputDocument::Query,
                pointer: self.pointer.clone(),
                line,
                column,
            }),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: {} (at `{}`)",
            self.code, self.message, self.pointer
        )
    }
}

impl std::error::Error for ParseError {}

/// An admission step of `menura-contracts` failed with one of the query codes it raises
/// (`E_INVALID_QUERY`, `EQRY-01`, `EQRY-08`).
impl From<AdmissionError> for ParseError {
    fn from(error: AdmissionError) -> Self {
        use menura_contracts::AdmissionCode;
        let code = match error.code {
            AdmissionCode::Eqry01 => ParseErrorCode::Eqry01,
            AdmissionCode::Eqry08 => ParseErrorCode::Eqry08,
            _ => ParseErrorCode::InvalidQuery,
        };
        Self {
            code,
            pointer: error.pointer,
            line_column: error.line_column,
            message: error.message,
            warnings: Vec::new(),
        }
    }
}

/// `WQRY-01`: the query uses the former contract name or the former extension namespace; it
/// is read as the new name (E1-2.4, CON-0.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseWarning {
    /// JSON Pointer of the former name: `/contract` or an extension's `data/namespace`.
    pub pointer: String,
    /// English description naming the former name and its replacement.
    pub message: String,
}

impl ParseWarning {
    /// The code as the specification writes it, `WQRY-01`.
    #[must_use]
    pub fn code(&self) -> &'static str {
        "WQRY-01"
    }

    /// The diagnostic of the compiled plan's `warnings` (PLN-1), located in the query.
    #[must_use]
    pub fn diagnostic(&self) -> Diagnostic {
        Diagnostic {
            code: self.code().to_owned(),
            message: self.message.clone(),
            location: Some(DiagnosticLocation {
                document: InputDocument::Query,
                pointer: self.pointer.clone(),
                line: None,
                column: None,
            }),
        }
    }
}

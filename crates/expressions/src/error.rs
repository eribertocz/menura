//! The error of the expression parser and its specification codes (E0F-8).

use std::fmt;

/// A range of bytes in the expression text, `start..end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    /// Offset of the first byte.
    pub start: usize,
    /// Offset just past the last byte.
    pub end: usize,
}

impl Span {
    /// The smallest span covering both `self` and `other`.
    #[must_use]
    pub fn to(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// The specification code of an expression error (E0F-8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExpressionCode {
    /// `E0F-01`: the expression cannot be parsed.
    Unparsable,
    /// `E0F-02`: a construct, function or operator outside the accepted subset (E0F-2.2).
    OutsideSubset,
}

impl ExpressionCode {
    /// The code as the specification writes it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unparsable => "E0F-01",
            Self::OutsideSubset => "E0F-02",
        }
    }
}

impl fmt::Display for ExpressionCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// What went wrong while reading an expression.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExpressionErrorKind {
    /// A character that starts no token. `E0F-01`.
    UnexpectedCharacter(char),
    /// A text literal without its closing quote. `E0F-01`.
    UnterminatedText,
    /// A delimited identifier without its closing quote. `E0F-01`.
    UnterminatedIdentifier,
    /// A `/*` comment without its closing `*/`. `E0F-01`.
    UnterminatedComment,
    /// A delimited identifier with no characters, `""`. `E0F-01`.
    EmptyIdentifier,
    /// A malformed number, or an exponent that does not fit. `E0F-01`.
    InvalidNumber(String),
    /// A `DATE` literal whose text is not a valid `YYYY-MM-DD` date. `E0F-01`.
    InvalidDate(String),
    /// A `TIMESTAMP` literal whose text is not a valid `YYYY-MM-DD HH:MM:SS[.fraction]`. `E0F-01`.
    InvalidTimestamp(String),
    /// A token where the grammar expects something else. `E0F-01`.
    UnexpectedToken {
        /// The token found, as written.
        found: String,
        /// What the grammar expects at that point.
        expected: &'static str,
    },
    /// The text ends where the grammar expects more. `E0F-01`.
    UnexpectedEnd {
        /// What the grammar expects at that point.
        expected: &'static str,
    },
    /// A subquery: `SELECT` or `EXISTS` (E0F-2.3). `E0F-02`.
    Subquery,
    /// A call of a function outside the accepted subset: non-deterministic, engine-specific or
    /// not listed (E0F-2.2, E0F-2.3). `E0F-02`.
    UnsupportedFunction(String),
    /// An operator outside the accepted subset, such as `!=` or `::`. `E0F-02`.
    UnsupportedOperator(String),
    /// A clause or form outside the accepted subset, such as `LIKE … ESCAPE`. `E0F-02`.
    UnsupportedConstruct(&'static str),
    /// A `CAST` target type outside the accepted subset. `E0F-02`.
    UnsupportedType(String),
    /// A function of the subset called with a number of arguments it does not take. `E0F-02`.
    WrongArgumentCount {
        /// The function, as the specification writes it.
        function: &'static str,
        /// The number of arguments written.
        found: usize,
    },
    /// A window function, `… OVER (…)`: not part of version 1 (E0F-2.4). `E0F-02`; in a
    /// metric, E0 reports `E_DEFERRED_WINDOWED_METRIC` instead (E0G-2.2).
    WindowFunction,
}

impl ExpressionErrorKind {
    /// The specification code of this kind (E0F-8).
    #[must_use]
    pub fn code(&self) -> ExpressionCode {
        match self {
            Self::UnexpectedCharacter(_)
            | Self::UnterminatedText
            | Self::UnterminatedIdentifier
            | Self::UnterminatedComment
            | Self::EmptyIdentifier
            | Self::InvalidNumber(_)
            | Self::InvalidDate(_)
            | Self::InvalidTimestamp(_)
            | Self::UnexpectedToken { .. }
            | Self::UnexpectedEnd { .. } => ExpressionCode::Unparsable,
            Self::Subquery
            | Self::UnsupportedFunction(_)
            | Self::UnsupportedOperator(_)
            | Self::UnsupportedConstruct(_)
            | Self::UnsupportedType(_)
            | Self::WrongArgumentCount { .. }
            | Self::WindowFunction => ExpressionCode::OutsideSubset,
        }
    }
}

impl fmt::Display for ExpressionErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedCharacter(character) => {
                write!(formatter, "unexpected character {character:?}")
            }
            Self::UnterminatedText => formatter.write_str("text literal without closing quote"),
            Self::UnterminatedIdentifier => {
                formatter.write_str("delimited identifier without closing quote")
            }
            Self::UnterminatedComment => formatter.write_str("comment without closing */"),
            Self::EmptyIdentifier => formatter.write_str("empty delimited identifier"),
            Self::InvalidNumber(text) => write!(formatter, "invalid number {text}"),
            Self::InvalidDate(text) => write!(formatter, "invalid DATE literal '{text}'"),
            Self::InvalidTimestamp(text) => write!(formatter, "invalid TIMESTAMP literal '{text}'"),
            Self::UnexpectedToken { found, expected } => {
                write!(formatter, "expected {expected}, found {found}")
            }
            Self::UnexpectedEnd { expected } => write!(
                formatter,
                "expected {expected}, found the end of the expression"
            ),
            Self::Subquery => formatter.write_str("subqueries are outside the accepted subset"),
            Self::UnsupportedFunction(name) => {
                write!(formatter, "function {name} is outside the accepted subset")
            }
            Self::UnsupportedOperator(operator) => write!(
                formatter,
                "operator {operator} is outside the accepted subset"
            ),
            Self::UnsupportedConstruct(construct) => {
                write!(formatter, "{construct} is outside the accepted subset")
            }
            Self::UnsupportedType(name) => write!(
                formatter,
                "CAST target type {name} is outside the accepted subset"
            ),
            Self::WrongArgumentCount { function, found } => {
                write!(formatter, "{function} does not take {found} arguments")
            }
            Self::WindowFunction => {
                formatter.write_str("window functions are not part of version 1")
            }
        }
    }
}

/// An expression that cannot be turned into a normalized tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExpressionError {
    kind: ExpressionErrorKind,
    span: Span,
}

impl ExpressionError {
    /// Builds an error of `kind` at `span`.
    #[must_use]
    pub fn new(kind: ExpressionErrorKind, span: Span) -> Self {
        Self { kind, span }
    }

    /// What went wrong.
    #[must_use]
    pub fn kind(&self) -> &ExpressionErrorKind {
        &self.kind
    }

    /// Where in the expression text, as byte offsets.
    #[must_use]
    pub fn span(&self) -> Span {
        self.span
    }

    /// The specification code (E0F-8).
    #[must_use]
    pub fn code(&self) -> ExpressionCode {
        self.kind.code()
    }
}

impl fmt::Display for ExpressionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at bytes {}..{}: {}",
            self.code(),
            self.span.start,
            self.span.end,
            self.kind
        )
    }
}

impl std::error::Error for ExpressionError {}

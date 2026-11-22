//! Positions in the SQL text and the failures located there (E1-4.5, E1-4.11).

use menura_representations::origin::{QueryLocation, TextPosition};

use crate::error::{ParseError, ParseErrorCode};
use crate::pointer::SQL;

/// The line and column, both from 1 and counted in code points, of the byte `offset` of
/// `sql` (E1-4.5). A line ends at each `\n`.
pub(super) fn text_position(sql: &str, offset: usize) -> Option<TextPosition> {
    let before = sql.get(..offset.min(sql.len())).unwrap_or_default();
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |newline| newline + 1);
    let column = before.get(line_start..).unwrap_or_default().chars().count() + 1;
    TextPosition::new(line, column)
}

/// The location of the element of the SQL text that starts at byte `offset`.
pub(super) fn location(sql: &str, offset: usize) -> QueryLocation {
    QueryLocation::new(SQL, text_position(sql, offset))
}

/// An error of the SQL text and the byte offset where a reader going from left to right
/// detects it, which orders the failures of E1-4.11 step 1 in text order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Failure {
    /// The offset where the failure is detected.
    pub(super) offset: usize,
    /// The error reported.
    pub(super) error: ParseError,
}

impl From<Failure> for ParseError {
    fn from(failure: Failure) -> Self {
        failure.error
    }
}

/// The failure with `code` at byte `offset`, its message ending with the line and column.
fn failure(code: ParseErrorCode, offset: usize, message: String) -> Failure {
    Failure {
        offset,
        error: ParseError::new(code, SQL, message),
    }
}

/// `EQRY-05` for a SQL text that does not parse at byte `offset` (E1-4.5).
pub(super) fn unparsable(sql: &str, offset: usize, problem: &str) -> Failure {
    failure(
        ParseErrorCode::Eqry05,
        offset,
        format!(
            "the SQL text does not parse at {}: {problem}",
            describe(sql, offset)
        ),
    )
}

/// `EQRY-05` for a construct outside the version 1 subset, at byte `offset` (E1-4).
pub(super) fn outside_subset(sql: &str, offset: usize, construct: &str) -> Failure {
    failure(
        ParseErrorCode::Eqry05,
        offset,
        format!(
            "{construct} is outside the SQL subset of version 1 (at {})",
            describe(sql, offset)
        ),
    )
}

/// `E0F-02` for an expression of the SQL text that parses with a construct outside the E0F-2
/// subset, at byte `offset`: the code of the same expression in the wide-table form (E1-4.12).
pub(super) fn invalid_expression(sql: &str, offset: usize, problem: &str) -> Failure {
    failure(
        ParseErrorCode::E0f02,
        offset,
        format!(
            "the expression at {} is not accepted: {problem}",
            describe(sql, offset)
        ),
    )
}

/// `EQRY-13` for the `SELECT` item at byte `offset` that is not a plain field reference and
/// has no alias.
pub(super) fn missing_alias(sql: &str, offset: usize, item: &str) -> Failure {
    failure(
        ParseErrorCode::Eqry13,
        offset,
        format!(
            "the SELECT item `{item}` (at {}) is not a plain field reference and has no alias; \
             add `AS <alias>`",
            describe(sql, offset)
        ),
    )
}

/// `line L, column C` of the byte `offset` of `sql`.
fn describe(sql: &str, offset: usize) -> String {
    text_position(sql, offset).map_or_else(String::new, |position| {
        format!("line {}, column {}", position.line(), position.column())
    })
}

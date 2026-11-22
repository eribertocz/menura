//! The expressions of a SQL statement, each read from its own slice of the text by the E0F-9
//! parser (E1-4 "SELECT items", "WHERE").

use std::ops::Range;

use menura_expressions::{ExpressionCode, ExpressionErrorKind, parse};
use menura_model::expression::Expression;

use super::lexer::{Token, TokenKind};
use super::position::{invalid_expression, outside_subset, unparsable};
use crate::error::ParseError;

/// `AGG`, which `MEASURE` is written as before the E0F-9 parser reads it, padded to the length
/// of `MEASURE` so that offsets in the text do not move.
const MEASURE_AS_AGG: &str = "AGG    ";

/// An expression of the statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SqlExpression {
    /// The parsed tree.
    pub(super) tree: Expression,
    /// The tree of the same text with every word in upper case, which compares two
    /// expressions as E0D-1.1 compares their regular identifiers.
    pub(super) key: Option<Expression>,
    /// The offset of its first byte in the SQL text.
    pub(super) start: usize,
    /// The text as written.
    pub(super) text: String,
}

/// Parses the tokens `range` of `tokens` as one expression of the E0F-2 language, with
/// `MEASURE(metric)` read as `AGG(metric)` (E1-4 "SELECT items").
///
/// # Errors
///
/// `EQRY-05` when the text does not parse (E1-4.5) or has a window function (E1-4
/// "Statement"); `E0F-02` when it parses with a construct outside the E0F-2 subset, the code
/// of the same expression in the wide-table form (E1-4.12).
pub(super) fn parse_expression(
    sql: &str,
    tokens: &[Token],
    range: Range<usize>,
) -> Result<SqlExpression, ParseError> {
    let slice = tokens.get(range).unwrap_or_default();
    let (Some(first), Some(last)) = (slice.first(), slice.last()) else {
        return Err(unparsable(sql, sql.len(), "expected an expression"));
    };
    let (start, end) = (first.start, last.end);
    let text = sql.get(start..end).unwrap_or_default().to_owned();
    let readable = rewritten(sql, slice, start, end, false);
    let tree = parse(&readable).map_err(|error| {
        let offset = start + error.span().start;
        match error.code() {
            ExpressionCode::Unparsable => unparsable(sql, offset, &error.kind().to_string()),
            ExpressionCode::OutsideSubset
                if matches!(error.kind(), ExpressionErrorKind::WindowFunction) =>
            {
                outside_subset(sql, offset, "a window function")
            }
            ExpressionCode::OutsideSubset => {
                invalid_expression(sql, offset, &error.kind().to_string())
            }
        }
    })?;
    let key = parse(&rewritten(sql, slice, start, end, true)).ok();
    Ok(SqlExpression {
        tree,
        key,
        start,
        text,
    })
}

/// The text `start..end` of `sql`, with each `MEASURE` before `(` written as `AGG`, and every
/// word in upper case when `upper_case` is set.
fn rewritten(sql: &str, slice: &[Token], start: usize, end: usize, upper_case: bool) -> String {
    let mut text = String::with_capacity(end - start);
    let mut copied = start;
    for (index, token) in slice.iter().enumerate() {
        if token.kind != TokenKind::Word {
            continue;
        }
        text.push_str(sql.get(copied..token.start).unwrap_or_default());
        let is_call = slice.get(index + 1).is_some_and(|next| next.is_symbol('('));
        let word = token.text(sql);
        if is_call && token.is_keyword(sql, "MEASURE") {
            text.push_str(MEASURE_AS_AGG);
        } else if upper_case {
            text.push_str(&word.to_uppercase());
        } else {
            text.push_str(word);
        }
        copied = token.end;
    }
    text.push_str(sql.get(copied..end).unwrap_or_default());
    text
}

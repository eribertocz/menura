//! The expressions of a SQL statement, each read from its own slice of the text by the E0F-9
//! parser (E1-4 "SELECT items", "WHERE").

use std::ops::Range;

use menura_expressions::{ExpressionCode, ExpressionErrorKind, parse};
use menura_model::expression::Expression;

use super::clauses::next_offset;
use super::lexer::{Token, TokenKind};
use super::position::{Failure, invalid_expression, outside_subset, unparsable};
use crate::error::ParseErrorCode;

/// `AGG`, which `MEASURE` is written as before the E0F-9 parser reads it, padded to the length
/// of `MEASURE` so that offsets in the text do not move.
const MEASURE_AS_AGG: &str = "AGG    ";

/// An expression of the statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SqlExpression {
    /// The parsed tree, or the E1-4.12 failure of a text that parses but is invalid, which is
    /// reported at the step of its clause (E1-4.11).
    pub(super) parsed: Result<Expression, Failure>,
    /// Its tokens.
    pub(super) tokens: Range<usize>,
    /// The offset of its first byte in the SQL text.
    pub(super) start: usize,
    /// The text as written.
    pub(super) text: String,
}

impl SqlExpression {
    /// The parsed tree.
    ///
    /// # Errors
    ///
    /// The E1-4.12 failure of an expression that parses but is invalid.
    pub(super) fn tree(&self) -> Result<&Expression, Failure> {
        self.parsed.as_ref().map_err(Clone::clone)
    }
}

/// Parses the tokens `range` of `tokens` as one expression of the E0F-2 language, with
/// `MEASURE(metric)` read as `AGG(metric)` (E1-4 "SELECT items"). The text is read up to byte
/// `end`.
///
/// # Errors
///
/// `EQRY-05` when the text does not parse (E1-4.5), detected where a reader of the whole text
/// would detect it, or has a window function (E1-4 "Statement"). A text that parses with a
/// construct outside the E0F-2 subset is not an error here: its `E0F-02` (E1-4.12) is kept in
/// [`SqlExpression::parsed`].
pub(super) fn parse_expression(
    sql: &str,
    tokens: &[Token],
    range: Range<usize>,
    end: usize,
) -> Result<SqlExpression, Failure> {
    let slice = tokens.get(range.clone()).unwrap_or_default();
    let (Some(first), Some(last)) = (slice.first(), slice.last()) else {
        let offset = next_offset(tokens, range.start, end);
        return Err(unparsable(sql, offset, "expected an expression"));
    };
    let (start, stop) = (first.start, last.end);
    let parsed = parse(&rewritten(sql, slice, start, stop, None)).map_err(|error| {
        let offset = start + error.span().start;
        match (error.code(), error.kind()) {
            (ExpressionCode::Unparsable, ExpressionErrorKind::UnexpectedEnd { .. }) => unparsable(
                sql,
                next_offset(tokens, range.end, end),
                &error.kind().to_string(),
            ),
            (ExpressionCode::Unparsable, kind) => unparsable(sql, offset, &kind.to_string()),
            (ExpressionCode::OutsideSubset, ExpressionErrorKind::WindowFunction) => {
                outside_subset(sql, offset, "a window function")
            }
            (ExpressionCode::OutsideSubset, kind) => {
                invalid_expression(sql, offset, &kind.to_string())
            }
        }
    });
    if let Err(failure) = &parsed
        && failure.error.code != ParseErrorCode::E0f02
    {
        return Err(failure.clone());
    }
    Ok(SqlExpression {
        parsed,
        tokens: range,
        start,
        text: sql.get(start..stop).unwrap_or_default().to_owned(),
    })
}

/// The tree that compares `expression` with another: its text with every word in upper case
/// (E0D-1.1 for regular identifiers) and without the qualifiers in `qualifiers`, the `FROM`
/// alias and dataset name as E0D-1.1 compares them (E1-4.11). `None` when it does not parse.
pub(super) fn comparison_key(
    sql: &str,
    tokens: &[Token],
    expression: &SqlExpression,
    qualifiers: &[String],
) -> Option<Expression> {
    let slice = tokens.get(expression.tokens.clone())?;
    let stop = slice.last()?.end;
    parse(&rewritten(
        sql,
        slice,
        expression.start,
        stop,
        Some(qualifiers),
    ))
    .ok()
}

/// The text `start..stop` of `sql`, with each `MEASURE` before `(` written as `AGG`. With
/// `key` qualifiers, the text of a comparison key: every word in upper case, and each qualifier
/// of `qualifiers` removed with its `.`.
fn rewritten(
    sql: &str,
    slice: &[Token],
    start: usize,
    stop: usize,
    key: Option<&[String]>,
) -> String {
    let mut text = String::with_capacity(stop - start);
    let mut copied = start;
    let mut index = 0;
    while let Some(token) = slice.get(index) {
        let removed =
            key.is_some_and(|qualifiers| is_removed_qualifier(sql, slice, index, qualifiers));
        let replacement = if removed {
            index += 1;
            Some(String::new())
        } else if token.kind != TokenKind::Word {
            None
        } else if slice.get(index + 1).is_some_and(|next| next.is_symbol('('))
            && token.is_keyword(sql, "MEASURE")
        {
            Some(MEASURE_AS_AGG.to_owned())
        } else {
            key.map(|_| token.text(sql).to_uppercase())
        };
        if let Some(replacement) = replacement {
            text.push_str(sql.get(copied..token.start).unwrap_or_default());
            text.push_str(&replacement);
            copied = slice.get(index).map_or(token.end, |last| last.end);
        }
        index += 1;
    }
    text.push_str(sql.get(copied..stop).unwrap_or_default());
    text
}

/// Whether `slice[index]` is a qualifier of `qualifiers` followed by `.` and a name, and not
/// itself after a `.`.
fn is_removed_qualifier(sql: &str, slice: &[Token], index: usize, qualifiers: &[String]) -> bool {
    let (Some(token), Some(dot), Some(name)) =
        (slice.get(index), slice.get(index + 1), slice.get(index + 2))
    else {
        return false;
    };
    let after_dot = index
        .checked_sub(1)
        .and_then(|before| slice.get(before))
        .is_some_and(|before| before.is_symbol('.'));
    let is_name =
        |token: &Token| matches!(token.kind, TokenKind::Word | TokenKind::QuotedIdentifier);
    is_name(token)
        && dot.is_symbol('.')
        && is_name(name)
        && !after_dot
        && qualifiers.contains(&name_key(sql, token))
}

/// The name a word or a delimited identifier writes, as E0D-1.1 compares it.
pub(super) fn name_key(sql: &str, token: &Token) -> String {
    let text = token.text(sql);
    match token.kind {
        TokenKind::QuotedIdentifier => text
            .get(1..text.len().saturating_sub(1))
            .unwrap_or_default()
            .replace("\"\"", "\""),
        _ => text.to_uppercase(),
    }
}

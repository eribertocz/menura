//! The text of typed literals and the `CAST` target types.

use semantic_compiler_model::expression::{
    CastTarget, DateValue, DecimalTarget, Literal, TimeOfDay, TimestampValue,
};

use super::{Parsed, Parser};
use crate::error::{ExpressionError, ExpressionErrorKind};
use crate::lexer::{Symbol, TokenKind};
use crate::vocabulary::{is_decimal_type, is_keyword, simple_cast_target};

/// The value of `DATE 'YYYY-MM-DD'`, or `None` when the text is not a valid date.
pub(super) fn date_value(text: &str) -> Option<DateValue> {
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    DateValue::new(
        digits(&text[0..4])?,
        digits(&text[5..7])?,
        digits(&text[8..10])?,
    )
}

/// The value of `TIMESTAMP 'YYYY-MM-DD HH:MM:SS[.fraction]'`, or `None` when the text is not a
/// valid timestamp without time zone.
pub(super) fn timestamp_value(text: &str) -> Option<TimestampValue> {
    let (date, time) = text.split_once(' ')?;
    let (clock, fraction) = time.split_once('.').unwrap_or((time, ""));
    let bytes = clock.as_bytes();
    if bytes.len() != 8
        || bytes[2] != b':'
        || bytes[5] != b':'
        || (time.contains('.') && fraction.is_empty())
    {
        return None;
    }
    Some(TimestampValue {
        date: date_value(date)?,
        time: TimeOfDay::new(
            digits(&clock[0..2])?,
            digits(&clock[3..5])?,
            digits(&clock[6..8])?,
            fraction,
        )?,
    })
}

/// A fixed-width group of ASCII digits as a number.
fn digits<T: std::str::FromStr>(text: &str) -> Option<T> {
    if text.bytes().all(|byte| byte.is_ascii_digit()) {
        text.parse().ok()
    } else {
        None
    }
}

impl Parser<'_> {
    /// The type after `AS` in a `CAST`.
    pub(super) fn cast_target(&mut self) -> Parsed<CastTarget> {
        let Some(TokenKind::Word(word)) = self.peek_kind_at(0).cloned() else {
            return Err(self.unexpected("a type"));
        };
        let start = self.next_span();
        self.position += 1;
        let target = if is_decimal_type(&word) {
            self.decimal_target()?
        } else if is_keyword(&word, "DOUBLE") && self.eat_keyword("PRECISION") {
            Some(CastTarget::DoublePrecision)
        } else {
            simple_cast_target(&word).filter(|_| !self.symbol_at(0, Symbol::LeftParenthesis))
        };
        match target {
            Some(target) if self.symbol_at(0, Symbol::RightParenthesis) => Ok(target),
            _ => {
                let end = self.next_span();
                let span = start.to(self.previous_span());
                let span = if matches!(self.peek_kind_at(0), Some(TokenKind::Word(_))) {
                    span.to(end)
                } else {
                    span
                };
                let name = self.text[span.start..span.end].to_owned();
                Err(ExpressionError::new(
                    ExpressionErrorKind::UnsupportedType(name),
                    span,
                ))
            }
        }
    }

    /// `(precision [, scale])` after `DECIMAL` or `NUMERIC`; `None` when missing or out of range.
    fn decimal_target(&mut self) -> Parsed<Option<CastTarget>> {
        if !self.eat_symbol(Symbol::LeftParenthesis) {
            return Ok(None);
        }
        let precision = self.small_integer()?;
        let scale = if self.eat_symbol(Symbol::Comma) {
            self.small_integer()?
        } else {
            Some(0)
        };
        self.expect_symbol(Symbol::RightParenthesis)?;
        Ok(precision
            .zip(scale)
            .and_then(|(precision, scale)| DecimalTarget::new(precision, scale))
            .map(CastTarget::Decimal))
    }

    /// An unsigned integer literal; `None` when it does not fit in a byte.
    fn small_integer(&mut self) -> Parsed<Option<u8>> {
        let Some(TokenKind::Number(Literal::Exact(number))) = self.peek_kind_at(0).cloned() else {
            return Err(self.unexpected("an integer"));
        };
        if !number.fraction_digits().is_empty() {
            return Err(self.unexpected("an integer"));
        }
        self.position += 1;
        Ok(number.integer_digits().parse().ok())
    }
}

//! The call forms of E0F-10 with their own syntax: `EXTRACT(part FROM x)`,
//! `DATE_PART('part', x)`, `DATE_TRUNC('unit', x)`, `DATEADD(unit, n, x)`,
//! `DATEDIFF(unit, start, end)` and `POSITION(a IN s)`. Parts and units become closed enums
//! (E0F-10.1); a part the subset leaves out, such as `MILLISECOND` (DEV-10), is `E0F-02`.

use menura_model::expression::{DateUnit, Expression, Literal, ScalarFunction};

use super::calls::exactly;
use super::{Parsed, Parser};
use crate::error::{ExpressionError, ExpressionErrorKind, Span};
use crate::lexer::{Symbol, TokenKind};
use crate::vocabulary::{date_unit, extract_part, is_keyword};

/// How a date unit may be written in a function's first argument.
#[derive(Clone, Copy, PartialEq, Eq)]
enum UnitSpelling {
    /// Only a text literal: `DATE_TRUNC('month', x)`.
    TextOnly,
    /// A bare word or a text literal: `DATEADD(month, 1, x)`.
    WordOrText,
}

impl Parser<'_> {
    /// A call with its own form named `word`, or `None` when `word` names none.
    pub(super) fn date_call(&mut self, word: &str, start: Span) -> Parsed<Option<Expression>> {
        let expression = if is_keyword(word, "EXTRACT") {
            self.extract()?
        } else if is_keyword(word, "POSITION") {
            self.position_call()?
        } else if is_keyword(word, "DATE_PART") {
            self.position += 2;
            let arguments = self.plain_arguments()?;
            let span = start.to(self.previous_span());
            let [part, operand] = exactly("DATE_PART", arguments, span)?;
            Expression::Extract {
                part: text_part(&part, extract_part, span)?,
                operand: Box::new(operand),
            }
        } else if is_keyword(word, "DATE_TRUNC") {
            self.position += 2;
            let arguments = self.plain_arguments()?;
            let span = start.to(self.previous_span());
            let [unit, operand] = exactly("DATE_TRUNC", arguments, span)?;
            Expression::DateTrunc {
                unit: unit_of(&unit, UnitSpelling::TextOnly, span)?,
                operand: Box::new(operand),
            }
        } else if is_keyword(word, "DATEADD") {
            self.position += 2;
            let arguments = self.plain_arguments()?;
            let span = start.to(self.previous_span());
            let [unit, amount, operand] = exactly("DATEADD", arguments, span)?;
            Expression::DateAdd {
                unit: unit_of(&unit, UnitSpelling::WordOrText, span)?,
                amount: Box::new(amount),
                operand: Box::new(operand),
            }
        } else if is_keyword(word, "DATEDIFF") {
            self.position += 2;
            let arguments = self.plain_arguments()?;
            let span = start.to(self.previous_span());
            let [unit, start_value, end] = exactly("DATEDIFF", arguments, span)?;
            Expression::DateDiff {
                unit: unit_of(&unit, UnitSpelling::WordOrText, span)?,
                start: Box::new(start_value),
                end: Box::new(end),
            }
        } else {
            return Ok(None);
        };
        Ok(Some(expression))
    }

    /// `EXTRACT(part FROM operand)`, with the part as a bare word.
    fn extract(&mut self) -> Parsed<Expression> {
        self.position += 2;
        let Some(TokenKind::Word(word)) = self.peek_kind_at(0).cloned() else {
            return Err(self.unexpected("a date part"));
        };
        let part = extract_part(&word).ok_or_else(|| {
            self.error_here(ExpressionErrorKind::UnsupportedDatePart(word.clone()))
        })?;
        self.position += 1;
        self.expect_keyword("FROM")?;
        let operand = self.expression()?;
        self.expect_symbol(Symbol::RightParenthesis)?;
        Ok(Expression::Extract {
            part,
            operand: Box::new(operand),
        })
    }

    /// `POSITION(needle IN haystack)`, stored like `CHARINDEX(needle, haystack)`.
    fn position_call(&mut self) -> Parsed<Expression> {
        self.position += 2;
        let needle = self.additive()?;
        self.expect_keyword("IN")?;
        let haystack = self.expression()?;
        self.expect_symbol(Symbol::RightParenthesis)?;
        Ok(Expression::Function {
            function: ScalarFunction::Position,
            arguments: vec![needle, haystack],
        })
    }
}

/// The text of a part written as a text literal, or as a bare word when `spelling` allows it.
fn part_text(argument: &Expression, spelling: UnitSpelling) -> Option<&str> {
    match argument {
        Expression::Literal(Literal::Text(text)) => Some(text),
        Expression::Reference(reference)
            if spelling == UnitSpelling::WordOrText
                && reference.qualifier.is_none()
                && !reference.name.is_delimited() =>
        {
            Some(reference.name.text())
        }
        _ => None,
    }
}

/// The part of `DATE_PART`, which must be a text literal.
fn text_part<T>(argument: &Expression, lookup: fn(&str) -> Option<T>, span: Span) -> Parsed<T> {
    let text = part_text(argument, UnitSpelling::TextOnly).ok_or_else(|| misspelled_part(span))?;
    lookup(text).ok_or_else(|| unsupported_part(text, span))
}

/// The unit of `DATE_TRUNC`, `DATEADD` or `DATEDIFF`.
fn unit_of(argument: &Expression, spelling: UnitSpelling, span: Span) -> Parsed<DateUnit> {
    let text = part_text(argument, spelling).ok_or_else(|| misspelled_part(span))?;
    date_unit(text).ok_or_else(|| unsupported_part(text, span))
}

/// A part written in a way E0F-10.1 does not allow, located at the call.
fn misspelled_part(span: Span) -> ExpressionError {
    ExpressionError::new(
        ExpressionErrorKind::UnsupportedConstruct(
            "a date part that is not written as E0F-10.1 says",
        ),
        span,
    )
}

/// A part E0F-10.1 does not list, located at the call.
fn unsupported_part(text: &str, span: Span) -> ExpressionError {
    ExpressionError::new(
        ExpressionErrorKind::UnsupportedDatePart(text.to_owned()),
        span,
    )
}

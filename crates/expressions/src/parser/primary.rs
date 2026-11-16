//! Primary expressions: literals, parenthesized expressions, references, `CASE`, `CAST`
//! and `CURRENT_*`.

use semantic_compiler_model::expression::{CaseBranch, Expression, Literal, Reference};

use super::literals::{date_value, time_value, timestamp_value};
use super::{Parsed, Parser};
use crate::error::{ExpressionError, ExpressionErrorKind};
use crate::lexer::{Symbol, TokenKind};
use crate::vocabulary::{is_keyword, is_reserved};

impl Parser<'_> {
    pub(super) fn primary(&mut self) -> Parsed<Expression> {
        let kind = self.peek_kind_at(0).cloned();
        match kind {
            Some(TokenKind::Number(literal)) => {
                self.position += 1;
                Ok(Expression::Literal(literal))
            }
            Some(TokenKind::Text(text)) => {
                self.position += 1;
                Ok(Expression::Literal(Literal::Text(text)))
            }
            Some(TokenKind::Symbol(Symbol::LeftParenthesis)) => {
                self.position += 1;
                let inner = self.expression()?;
                self.expect_symbol(Symbol::RightParenthesis)?;
                Ok(inner)
            }
            Some(TokenKind::Word(word)) => self.word(&word),
            Some(TokenKind::QuotedIdentifier(_)) => self.reference().map(Expression::Reference),
            _ => Err(self.unexpected("an expression")),
        }
    }

    /// A primary expression that starts with a word.
    fn word(&mut self, word: &str) -> Parsed<Expression> {
        if is_keyword(word, "CAST") {
            return self.cast();
        }
        if is_keyword(word, "CASE") {
            return self.case();
        }
        if let Some(current) = current_moment(word) {
            self.position += 1;
            if self.symbol_at(0, Symbol::LeftParenthesis)
                && self.symbol_at(1, Symbol::RightParenthesis)
            {
                self.position += 2;
            }
            return Ok(current);
        }
        if self.symbol_at(1, Symbol::LeftParenthesis) && !is_reserved(word) {
            return self.call(word);
        }
        if let Some(literal) = keyword_literal(word) {
            self.position += 1;
            return Ok(Expression::Literal(literal));
        }
        if let Some(TokenKind::Text(text)) = self.peek_kind_at(1).cloned()
            && let Some(literal) = self.typed_literal(word, &text)?
        {
            return Ok(Expression::Literal(literal));
        }
        if is_reserved(word) {
            return Err(self.unexpected("an expression"));
        }
        self.reference().map(Expression::Reference)
    }

    /// `DATE 'text'`, `TIME 'text'`, `TIMESTAMP 'text'` or `TIMESTAMP_NTZ 'text'`; `None` when
    /// `word` is none of these.
    fn typed_literal(&mut self, word: &str, text: &str) -> Parsed<Option<Literal>> {
        let literal = if is_keyword(word, "DATE") {
            date_value(text)
                .map(Literal::Date)
                .ok_or_else(|| ExpressionErrorKind::InvalidDate(text.to_owned()))
        } else if is_keyword(word, "TIME") {
            time_value(text)
                .map(Literal::Time)
                .ok_or_else(|| ExpressionErrorKind::InvalidTime(text.to_owned()))
        } else if is_keyword(word, "TIMESTAMP") || is_keyword(word, "TIMESTAMP_NTZ") {
            timestamp_value(text)
                .map(Literal::Timestamp)
                .ok_or_else(|| ExpressionErrorKind::InvalidTimestamp(text.to_owned()))
        } else {
            return Ok(None);
        };
        let start = self.next_span();
        self.position += 2;
        let span = start.to(self.previous_span());
        literal
            .map(Some)
            .map_err(|kind| ExpressionError::new(kind, span))
    }

    /// `name` or `qualifier.name`; a third part or a following `(` is outside the subset.
    pub(super) fn reference(&mut self) -> Parsed<Reference> {
        let start = self.next_span();
        let first = self.identifier()?;
        let reference = if self.eat_symbol(Symbol::Period) {
            Reference {
                qualifier: Some(first),
                name: self.identifier()?,
            }
        } else {
            Reference {
                qualifier: None,
                name: first,
            }
        };
        if self.symbol_at(0, Symbol::Period) {
            return Err(self.error_here(ExpressionErrorKind::UnsupportedConstruct(
                "a reference with more than two parts",
            )));
        }
        if self.symbol_at(0, Symbol::LeftParenthesis) {
            let span = start.to(self.previous_span());
            let name = self.text[span.start..span.end].to_owned();
            return Err(ExpressionError::new(
                ExpressionErrorKind::UnsupportedFunction(name),
                span,
            ));
        }
        Ok(reference)
    }

    /// `CASE [operand] WHEN … THEN … [WHEN …] [ELSE …] END`.
    fn case(&mut self) -> Parsed<Expression> {
        self.position += 1;
        let operand = if self.keyword_at(0, "WHEN") {
            None
        } else {
            Some(Box::new(self.expression()?))
        };
        let mut branches = Vec::new();
        while self.eat_keyword("WHEN") {
            let condition = self.expression()?;
            self.expect_keyword("THEN")?;
            let result = self.expression()?;
            branches.push(CaseBranch { condition, result });
        }
        if branches.is_empty() {
            return Err(self.unexpected("WHEN"));
        }
        let otherwise = if self.eat_keyword("ELSE") {
            Some(Box::new(self.expression()?))
        } else {
            None
        };
        self.expect_keyword("END")?;
        Ok(Expression::Case {
            operand,
            branches,
            otherwise,
        })
    }

    /// `CAST(operand AS type)`.
    fn cast(&mut self) -> Parsed<Expression> {
        self.position += 1;
        self.expect_symbol(Symbol::LeftParenthesis)?;
        let operand = self.expression()?;
        self.expect_keyword("AS")?;
        let target = self.cast_target()?;
        self.expect_symbol(Symbol::RightParenthesis)?;
        Ok(Expression::Cast {
            operand: Box::new(operand),
            target,
        })
    }
}

/// `CURRENT_DATE`, `CURRENT_TIME` or `CURRENT_TIMESTAMP` (E0F-10.1), if `word` is one.
fn current_moment(word: &str) -> Option<Expression> {
    if is_keyword(word, "CURRENT_DATE") {
        Some(Expression::CurrentDate)
    } else if is_keyword(word, "CURRENT_TIME") {
        Some(Expression::CurrentTime)
    } else if is_keyword(word, "CURRENT_TIMESTAMP") {
        Some(Expression::CurrentTimestamp)
    } else {
        None
    }
}

fn keyword_literal(word: &str) -> Option<Literal> {
    if is_keyword(word, "NULL") {
        Some(Literal::Null)
    } else if is_keyword(word, "TRUE") {
        Some(Literal::Boolean(true))
    } else if is_keyword(word, "FALSE") {
        Some(Literal::Boolean(false))
    } else {
        None
    }
}

//! Recursive-descent parser from tokens to the normalized tree (E0F-2.2, E0H-1.3).
//!
//! Normalization happens by construction: the lexer drops whitespace and comments, keywords are
//! matched without regard to ASCII case, and parentheses produce no node.

mod aggregates;
mod calls;
mod dates;
mod literals;
mod operators;
mod primary;

use semantic_compiler_model::expression::{Expression, Identifier};

use crate::error::{ExpressionError, ExpressionErrorKind, Span};
use crate::lexer::{Symbol, Token, TokenKind, tokenize};
use crate::vocabulary::{is_keyword, is_reserved};

/// The result of one parsing step.
type Parsed<T> = Result<T, ExpressionError>;

/// Parses `ANSI_SQL` expression text into its normalized tree.
///
/// # Errors
///
/// Returns the first problem from left to right (E0F-9.1): `E0F-01` when the grammar rejects
/// the text, `E0F-02` when it accepts a construct outside the subset (E0F-2.2, E0F-2.6),
/// including window functions (E0F-2.4, [`ExpressionErrorKind::WindowFunction`]).
///
/// # Examples
///
/// ```
/// use semantic_compiler_expressions::{ExpressionCode, parse};
/// assert!(parse("COALESCE(orders.discount, 0)").is_ok());
/// assert_eq!(parse("RANDOM()").unwrap_err().code(), ExpressionCode::OutsideSubset);
/// assert_eq!(parse("SUM(").unwrap_err().code(), ExpressionCode::Unparsable);
/// ```
pub fn parse(text: &str) -> Result<Expression, ExpressionError> {
    let mut parser = Parser {
        text,
        tokens: tokenize(text),
        position: 0,
    };
    let expression = parser.expression()?;
    if parser.peek().is_some() {
        return Err(parser.unexpected("an operator or the end of the expression"));
    }
    Ok(expression)
}

/// The parser's state: the tokens and the index of the next one.
struct Parser<'text> {
    text: &'text str,
    tokens: Vec<Token>,
    position: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn peek_kind_at(&self, offset: usize) -> Option<&TokenKind> {
        self.tokens
            .get(self.position + offset)
            .map(|token| &token.kind)
    }

    /// The span of the next token, or an empty span at the end of the text.
    fn next_span(&self) -> Span {
        self.peek().map_or(
            Span {
                start: self.text.len(),
                end: self.text.len(),
            },
            |token| token.span,
        )
    }

    /// The span of the token just consumed.
    fn previous_span(&self) -> Span {
        self.position
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
            .map_or(Span { start: 0, end: 0 }, |token| token.span)
    }

    fn symbol_at(&self, offset: usize, symbol: Symbol) -> bool {
        self.peek_kind_at(offset) == Some(&TokenKind::Symbol(symbol))
    }

    fn keyword_at(&self, offset: usize, keyword: &str) -> bool {
        matches!(self.peek_kind_at(offset), Some(TokenKind::Word(word)) if is_keyword(word, keyword))
    }

    fn eat_symbol(&mut self, symbol: Symbol) -> bool {
        let found = self.symbol_at(0, symbol);
        if found {
            self.position += 1;
        }
        found
    }

    fn eat_keyword(&mut self, keyword: &str) -> bool {
        let found = self.keyword_at(0, keyword);
        if found {
            self.position += 1;
        }
        found
    }

    fn expect_symbol(&mut self, symbol: Symbol) -> Parsed<()> {
        if self.eat_symbol(symbol) {
            Ok(())
        } else {
            Err(self.unexpected(symbol.text()))
        }
    }

    fn expect_keyword(&mut self, keyword: &'static str) -> Parsed<()> {
        if self.eat_keyword(keyword) {
            Ok(())
        } else {
            Err(self.unexpected(keyword))
        }
    }

    /// The error for the next token when the grammar expects `expected` there.
    ///
    /// A lexical error token reports itself.
    fn unexpected(&self, expected: &'static str) -> ExpressionError {
        let span = self.next_span();
        let kind = match self.peek().map(|token| &token.kind) {
            None => ExpressionErrorKind::UnexpectedEnd { expected },
            Some(TokenKind::Invalid(kind)) => kind.clone(),
            Some(_) => ExpressionErrorKind::UnexpectedToken {
                found: self.text[span.start..span.end].to_owned(),
                expected,
            },
        };
        ExpressionError::new(kind, span)
    }

    /// An error of `kind` at the next token.
    fn error_here(&self, kind: ExpressionErrorKind) -> ExpressionError {
        ExpressionError::new(kind, self.next_span())
    }

    /// A regular identifier that is not reserved, or a delimited identifier.
    fn identifier(&mut self) -> Parsed<Identifier> {
        let identifier = match self.peek_kind_at(0) {
            Some(TokenKind::Word(word)) if !is_reserved(word) => Identifier::regular(word),
            Some(TokenKind::QuotedIdentifier(text)) => Identifier::delimited(text),
            _ => None,
        };
        let identifier = identifier.ok_or_else(|| self.unexpected("an identifier"))?;
        self.position += 1;
        Ok(identifier)
    }
}

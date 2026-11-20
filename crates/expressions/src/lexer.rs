//! Splits expression text into tokens (ISO SQL lexical rules, E0F-2.1).
//!
//! Whitespace and comments (`-- …` to the end of the line, `/* … */`) separate tokens and leave
//! no trace. A lexical error ends the token list with an [`TokenKind::Invalid`] token, so the
//! parser reports errors strictly from left to right.

use menura_model::expression::{
    DecimalLiteral, FloatLiteral, IntegerLiteral, Literal, MAXIMUM_EXACT_DIGITS,
};

use crate::error::{ExpressionErrorKind, Span};

/// A punctuation or operator token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Symbol {
    LeftParenthesis,
    RightParenthesis,
    Comma,
    Period,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Concatenate,
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}

impl Symbol {
    /// The symbol as written.
    pub(crate) fn text(self) -> &'static str {
        match self {
            Self::LeftParenthesis => "(",
            Self::RightParenthesis => ")",
            Self::Comma => ",",
            Self::Period => ".",
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Star => "*",
            Self::Slash => "/",
            Self::Percent => "%",
            Self::Concatenate => "||",
            Self::Equal => "=",
            Self::NotEqual => "<>",
            Self::Less => "<",
            Self::LessOrEqual => "<=",
            Self::Greater => ">",
            Self::GreaterOrEqual => ">=",
        }
    }
}

/// What a token is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TokenKind {
    /// A regular identifier or keyword, as written.
    Word(String),
    /// A delimited identifier, quotes removed and `""` undoubled.
    QuotedIdentifier(String),
    /// A text literal, quotes removed and `''` undoubled.
    Text(String),
    /// A number literal.
    Number(Literal),
    /// Punctuation or an operator.
    Symbol(Symbol),
    /// A lexical error; always the last token.
    Invalid(ExpressionErrorKind),
}

/// One token and where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Token {
    pub(crate) kind: TokenKind,
    pub(crate) span: Span,
}

/// Splits `text` into tokens. The list stops after the first [`TokenKind::Invalid`] token.
pub(crate) fn tokenize(text: &str) -> Vec<Token> {
    let mut lexer = Lexer { text, position: 0 };
    let mut tokens = Vec::new();
    loop {
        if let Err(error) = lexer.skip_separators() {
            tokens.push(error);
            break;
        }
        let Some(token) = lexer.next_token() else {
            break;
        };
        let invalid = matches!(token.kind, TokenKind::Invalid(_));
        tokens.push(token);
        if invalid {
            break;
        }
    }
    tokens
}

struct Lexer<'text> {
    text: &'text str,
    position: usize,
}

impl Lexer<'_> {
    fn rest(&self) -> &str {
        &self.text[self.position..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn token(&self, kind: TokenKind, start: usize) -> Token {
        Token {
            kind,
            span: Span {
                start,
                end: self.position,
            },
        }
    }

    /// Skips whitespace and comments; an unterminated `/*` gives an invalid token.
    fn skip_separators(&mut self) -> Result<(), Token> {
        loop {
            let rest = self.rest();
            if let Some(character) = rest
                .chars()
                .next()
                .filter(|character| character.is_whitespace())
            {
                self.position += character.len_utf8();
            } else if rest.starts_with("--") {
                self.position += rest.find('\n').unwrap_or(rest.len());
            } else if let Some(comment) = rest.strip_prefix("/*") {
                let start = self.position;
                let Some(length) = comment.find("*/") else {
                    self.position = self.text.len();
                    return Err(self.token(
                        TokenKind::Invalid(ExpressionErrorKind::UnterminatedComment),
                        start,
                    ));
                };
                self.position += length + 4;
            } else {
                return Ok(());
            }
        }
    }

    fn next_token(&mut self) -> Option<Token> {
        let start = self.position;
        let character = self.peek()?;
        let kind = if character.is_alphabetic() || character == '_' {
            TokenKind::Word(self.take_word().to_owned())
        } else if character.is_ascii_digit()
            || (character == '.'
                && self.rest()[1..].starts_with(|next: char| next.is_ascii_digit()))
        {
            self.number()
        } else if character == '\'' {
            self.quoted('\'', ExpressionErrorKind::UnterminatedText)
                .map_or_else(TokenKind::Invalid, TokenKind::Text)
        } else if character == '"' {
            self.quoted_identifier()
        } else {
            self.symbol(character)
        };
        Some(self.token(kind, start))
    }

    fn take_word(&mut self) -> &str {
        let start = self.position;
        let length = self
            .rest()
            .find(|character: char| !(character.is_alphanumeric() || character == '_'))
            .unwrap_or(self.rest().len());
        self.position += length;
        &self.text[start..self.position]
    }

    fn take_digits(&mut self) -> &str {
        let start = self.position;
        let length = self
            .rest()
            .find(|character: char| !character.is_ascii_digit())
            .unwrap_or(self.rest().len());
        self.position += length;
        &self.text[start..self.position]
    }

    /// `digits [. digits] [E [+|-] digits]`, or `. digits [E …]` (E0F-9.3).
    fn number(&mut self) -> TokenKind {
        let start = self.position;
        let integer_digits = self.take_digits().to_owned();
        let point = self.rest().starts_with('.');
        if point {
            self.position += 1;
        }
        let fraction_digits = self.take_digits().to_owned();
        let exponent = if self.rest().starts_with(['e', 'E']) {
            self.position += 1;
            Some(self.exponent())
        } else {
            None
        };
        let glued = self
            .peek()
            .is_some_and(|next| next.is_alphanumeric() || next == '_' || next == '.');
        if glued {
            self.take_word();
        }
        let text = &self.text[start..self.position];
        let invalid =
            |kind: fn(String) -> ExpressionErrorKind| TokenKind::Invalid(kind(text.to_owned()));
        if glued {
            return invalid(ExpressionErrorKind::InvalidNumber);
        }
        match exponent {
            Some(Some(exponent)) => FloatLiteral::new(&integer_digits, &fraction_digits, exponent)
                .map_or_else(
                    || invalid(ExpressionErrorKind::InvalidNumber),
                    |number| TokenKind::Number(Literal::Float(number)),
                ),
            Some(None) => invalid(ExpressionErrorKind::InvalidNumber),
            None if integer_digits.len() + fraction_digits.len() > MAXIMUM_EXACT_DIGITS => {
                invalid(ExpressionErrorKind::NumberOutOfRange)
            }
            None if point => DecimalLiteral::new(&integer_digits, &fraction_digits).map_or_else(
                || invalid(ExpressionErrorKind::InvalidNumber),
                |number| TokenKind::Number(Literal::Decimal(number)),
            ),
            None => IntegerLiteral::new(&integer_digits).map_or_else(
                || invalid(ExpressionErrorKind::InvalidNumber),
                |number| TokenKind::Number(Literal::Integer(number)),
            ),
        }
    }

    /// The exponent after `E`: an optional sign and digits that fit in an `i32`.
    fn exponent(&mut self) -> Option<i32> {
        let negative = self.rest().starts_with('-');
        if self.rest().starts_with(['+', '-']) {
            self.position += 1;
        }
        let digits = self.take_digits();
        let magnitude: i32 = digits.parse().ok()?;
        Some(if negative { -magnitude } else { magnitude })
    }

    /// Text between `quote` characters, with a doubled quote standing for one.
    fn quoted(
        &mut self,
        quote: char,
        unterminated: ExpressionErrorKind,
    ) -> Result<String, ExpressionErrorKind> {
        self.position += quote.len_utf8();
        let mut content = String::new();
        loop {
            let Some(character) = self.peek() else {
                return Err(unterminated);
            };
            self.position += character.len_utf8();
            if character != quote {
                content.push(character);
            } else if self.peek() == Some(quote) {
                self.position += quote.len_utf8();
                content.push(quote);
            } else {
                return Ok(content);
            }
        }
    }

    fn quoted_identifier(&mut self) -> TokenKind {
        match self.quoted('"', ExpressionErrorKind::UnterminatedIdentifier) {
            Ok(content) if content.is_empty() => {
                TokenKind::Invalid(ExpressionErrorKind::EmptyIdentifier)
            }
            Ok(content) => TokenKind::QuotedIdentifier(content),
            Err(kind) => TokenKind::Invalid(kind),
        }
    }

    fn symbol(&mut self, character: char) -> TokenKind {
        let two = self.rest().get(..2).and_then(two_character_symbol);
        if let Some(symbol) = two {
            self.position += 2;
            return TokenKind::Symbol(symbol);
        }
        self.position += character.len_utf8();
        one_character_symbol(character).map_or(
            TokenKind::Invalid(ExpressionErrorKind::UnexpectedCharacter(character)),
            TokenKind::Symbol,
        )
    }
}

fn two_character_symbol(text: &str) -> Option<Symbol> {
    Some(match text {
        "||" => Symbol::Concatenate,
        "<>" | "!=" => Symbol::NotEqual,
        "<=" => Symbol::LessOrEqual,
        ">=" => Symbol::GreaterOrEqual,
        _ => return None,
    })
}

fn one_character_symbol(character: char) -> Option<Symbol> {
    Some(match character {
        '(' => Symbol::LeftParenthesis,
        ')' => Symbol::RightParenthesis,
        ',' => Symbol::Comma,
        '.' => Symbol::Period,
        '+' => Symbol::Plus,
        '-' => Symbol::Minus,
        '*' => Symbol::Star,
        '/' => Symbol::Slash,
        '%' => Symbol::Percent,
        '=' => Symbol::Equal,
        '<' => Symbol::Less,
        '>' => Symbol::Greater,
        _ => return None,
    })
}

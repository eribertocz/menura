//! The tokens of a SQL text: enough to find its clauses and items (E1-4). Each expression is
//! then read by the E0F-9 parser of `menura-expressions`, from its own slice of the text.

/// What a token is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TokenKind {
    /// A keyword, a function name or a regular identifier.
    Word,
    /// A delimited identifier, `"…"`.
    QuotedIdentifier,
    /// A text literal, `'…'`.
    Text,
    /// A number literal.
    Number,
    /// Any other character, such as `(`, `,` or an operator character.
    Symbol(char),
}

/// A token and the bytes `start..end` it covers in the SQL text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Token {
    /// What the token is.
    pub(super) kind: TokenKind,
    /// Offset of its first byte.
    pub(super) start: usize,
    /// Offset just past its last byte.
    pub(super) end: usize,
}

impl Token {
    /// The text of the token.
    pub(super) fn text<'text>(&self, sql: &'text str) -> &'text str {
        sql.get(self.start..self.end).unwrap_or_default()
    }

    /// Whether the token is the keyword `keyword`, compared case-insensitively (E0F-9.3).
    pub(super) fn is_keyword(&self, sql: &str, keyword: &str) -> bool {
        self.kind == TokenKind::Word && self.text(sql).eq_ignore_ascii_case(keyword)
    }

    /// Whether the token is the character `symbol`.
    pub(super) fn is_symbol(&self, symbol: char) -> bool {
        self.kind == TokenKind::Symbol(symbol)
    }
}

/// A text the tokens cannot cover: the byte offset where the failing element starts, and what
/// it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LexicalError {
    /// Offset of the first byte of the element.
    pub(super) offset: usize,
    /// The element, such as "text literal".
    pub(super) element: &'static str,
}

/// The tokens of `sql`, without whitespace and comments (E0F-9.3: `-- …` and `/* … */`, which
/// do not nest).
pub(super) fn tokenize(sql: &str) -> Result<Vec<Token>, LexicalError> {
    let mut tokens = Vec::new();
    let mut start = 0;
    while let Some(character) = sql.get(start..).and_then(|rest| rest.chars().next()) {
        let rest = sql.get(start..).unwrap_or_default();
        let error = |element| LexicalError {
            offset: start,
            element,
        };
        let length = if let Some(length) = skipped_length(rest, character).map_err(error)? {
            length
        } else {
            let (kind, length) = token_at(rest, character).map_err(error)?;
            tokens.push(Token {
                kind,
                start,
                end: start + length,
            });
            length
        };
        start += length;
    }
    Ok(tokens)
}

/// The length of the whitespace or comment that `rest`, whose first character is `first`,
/// starts with; `None` when it starts a token.
fn skipped_length(rest: &str, first: char) -> Result<Option<usize>, &'static str> {
    if first.is_whitespace() {
        return Ok(Some(first.len_utf8()));
    }
    if rest.starts_with("--") {
        return Ok(Some(rest.find('\n').unwrap_or(rest.len())));
    }
    if rest.starts_with("/*") {
        let end = rest.get(2..).and_then(|after| after.find("*/"));
        return end.map(|end| Some(end + 4)).ok_or("block comment");
    }
    Ok(None)
}

/// The token that starts `rest`, whose first character is `first`, and its length in bytes;
/// the unterminated element otherwise.
fn token_at(rest: &str, first: char) -> Result<(TokenKind, usize), &'static str> {
    if first == '\'' {
        return quoted_length(rest, '\'')
            .map(|length| (TokenKind::Text, length))
            .ok_or("text literal");
    }
    if first == '"' {
        return quoted_length(rest, '"')
            .map(|length| (TokenKind::QuotedIdentifier, length))
            .ok_or("delimited identifier");
    }
    if first.is_alphabetic() || first == '_' {
        let length = rest
            .find(|character: char| !(character.is_alphanumeric() || character == '_'))
            .unwrap_or(rest.len());
        return Ok((TokenKind::Word, length));
    }
    let starts_number = first.is_ascii_digit()
        || (first == '.' && rest[1..].starts_with(|character: char| character.is_ascii_digit()));
    if starts_number {
        return Ok((TokenKind::Number, number_length(rest)));
    }
    Ok((TokenKind::Symbol(first), first.len_utf8()))
}

/// The length of the literal or identifier that `rest` starts with, delimited by `quote` and
/// with a doubled `quote` standing for one; `None` when it has no closing `quote`.
fn quoted_length(rest: &str, quote: char) -> Option<usize> {
    let mut characters = rest.char_indices().skip(1).peekable();
    while let Some((offset, character)) = characters.next() {
        if character != quote {
            continue;
        }
        if characters.peek().is_some_and(|&(_, next)| next == quote) {
            characters.next();
        } else {
            return Some(offset + 1);
        }
    }
    None
}

/// The length of the number that `rest` starts with: digits, an optional fraction and an
/// optional exponent. A malformed number is left to the expression parser.
fn number_length(rest: &str) -> usize {
    let bytes = rest.as_bytes();
    let mut length = 0;
    while length < bytes.len() && (bytes[length].is_ascii_digit() || bytes[length] == b'.') {
        length += 1;
    }
    if length < bytes.len() && (bytes[length] == b'e' || bytes[length] == b'E') {
        length += 1;
        if length < bytes.len() && (bytes[length] == b'+' || bytes[length] == b'-') {
            length += 1;
        }
        while length < bytes.len() && bytes[length].is_ascii_digit() {
            length += 1;
        }
    }
    length
}

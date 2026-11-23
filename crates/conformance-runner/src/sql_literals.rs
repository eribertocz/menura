//! A small lexer that finds the literals of a SQL text: its string literals and its number
//! tokens, each with its byte offset. It reads both the expressions of a query (its `filter`,
//! its `filters` and its SQL form) and the `sql` of a compiled plan.

use std::ops::Range;

/// The kind of a literal found in a SQL text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LiteralKind {
    /// A string literal, `'…'`, with `''` for a quote.
    Text,
    /// A number token, such as `5` or `10.50`, outside any identifier or placeholder.
    Number,
}

/// A literal of a SQL text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Literal {
    /// Whether it is a string literal or a number token.
    pub(crate) kind: LiteralKind,
    /// Its value: the unquoted text of a string literal, or the normalized digits of a number
    /// (see [`normalized_number`]).
    pub(crate) value: String,
    /// The byte offset where it starts in the text.
    pub(crate) offset: usize,
    /// The byte offset just after it in the text.
    pub(crate) end: usize,
    /// The keyword or function name written just before it, upper-cased, when there is one:
    /// `LIMIT` before `LIMIT 10`.
    pub(crate) preceding_word: Option<String>,
    /// The function whose argument it is, upper-cased, and the argument's index from 0:
    /// `("ROUND", 1)` for the `2` of `ROUND(x, 2)`.
    pub(crate) call: Option<(String, usize)>,
    /// Whether it is a whole item of a `GROUP BY`, `ORDER BY` or `PARTITION BY` list, which
    /// names a column by position: `GROUP BY 1`.
    pub(crate) ordinal: bool,
}

/// The literals of `text`, in order. Double-quoted identifiers, positional placeholders (`$1`)
/// and the digits of identifiers (`col1`) are skipped.
pub(crate) fn literals(text: &str) -> Vec<Literal> {
    let mut lexer = Lexer::default();
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        index = lexer.step(text, index);
    }
    lexer.finish();
    lexer.found
}

/// The state of [`literals`] while it reads a text.
#[derive(Default)]
struct Lexer {
    /// The literals found so far.
    found: Vec<Literal>,
    /// The word just read, upper-cased, until a token other than `(` follows it.
    last_word: Option<String>,
    /// Whether the last significant token was `BY` or a comma of a `BY` list.
    after_by_separator: bool,
    /// The open parentheses: the function each one calls, if any, and the argument being read.
    calls: Vec<(Option<String>, usize)>,
    /// Whether the clause being read at each depth is a `BY` list; index 0 is the top level.
    by_lists: Vec<bool>,
    /// A number that follows a `BY` separator, kept until the next token says whether it is a
    /// whole list item.
    pending_item: Option<usize>,
}

/// The words that end a `BY` list.
const CLAUSE_WORDS: [&str; 10] = [
    "SELECT", "FROM", "WHERE", "HAVING", "LIMIT", "OFFSET", "QUALIFY", "WINDOW", "UNION", "ON",
];

impl Lexer {
    /// Reads the token at `index` and returns the offset after it.
    fn step(&mut self, text: &str, index: usize) -> usize {
        let bytes = text.as_bytes();
        let byte = bytes[index];
        if byte.is_ascii_whitespace() {
            return index + 1;
        }
        let item_ended =
            matches!(byte, b',' | b')') || is_word_byte(byte) && !byte.is_ascii_digit();
        self.close_pending_item(item_ended);
        if byte == b'\'' {
            let (value, end) = quoted(text, index, '\'');
            self.push(LiteralKind::Text, value, index..end);
            return end;
        }
        if byte.is_ascii_digit() || (byte == b'.' && next_is_digit(bytes, index)) {
            let end = number_end(bytes, index);
            let follows_separator = self.after_by_separator && self.in_by_list();
            self.push(
                LiteralKind::Number,
                normalized_number(&text[index..end]),
                index..end,
            );
            if follows_separator {
                self.pending_item = Some(self.found.len() - 1);
            }
            return end;
        }
        if byte == b'"' || byte == b'$' {
            self.other_token();
            return if byte == b'"' {
                quoted(text, index, '"').1
            } else {
                word_end(bytes, index + 1)
            };
        }
        if is_word_byte(byte) {
            let end = word_end(bytes, index);
            self.word(text[index..end].to_ascii_uppercase());
            return end;
        }
        self.punctuation(byte);
        index + 1
    }

    fn finish(&mut self) {
        self.close_pending_item(true);
    }

    /// Marks the pending number as a whole `BY` list item when the token after it ends the item.
    fn close_pending_item(&mut self, item_ended: bool) {
        if let Some(position) = self.pending_item.take()
            && item_ended
            && let Some(literal) = self.found.get_mut(position)
        {
            literal.ordinal = true;
        }
    }

    fn push(&mut self, kind: LiteralKind, value: String, span: Range<usize>) {
        let call = self
            .calls
            .last()
            .and_then(|(function, argument)| function.clone().map(|name| (name, *argument)));
        self.found.push(Literal {
            kind,
            value,
            offset: span.start,
            end: span.end,
            preceding_word: self.last_word.take(),
            call,
            ordinal: false,
        });
        self.after_by_separator = false;
    }

    fn word(&mut self, word: String) {
        self.after_by_separator = word == "BY";
        let depth = self.calls.len();
        if self.by_lists.len() <= depth {
            self.by_lists.resize(depth + 1, false);
        }
        if word == "BY" {
            self.by_lists[depth] = true;
        } else if CLAUSE_WORDS.contains(&word.as_str()) {
            self.by_lists[depth] = false;
        }
        self.last_word = Some(word);
    }

    fn other_token(&mut self) {
        self.last_word = None;
        self.after_by_separator = false;
    }

    fn punctuation(&mut self, byte: u8) {
        match byte {
            b'(' => {
                self.calls.push((self.last_word.take(), 0));
                self.by_lists.truncate(self.calls.len());
                self.after_by_separator = false;
            }
            b')' => {
                self.calls.pop();
                self.other_token();
            }
            b',' => {
                if let Some((_, argument)) = self.calls.last_mut() {
                    *argument += 1;
                }
                self.last_word = None;
                self.after_by_separator = self.in_by_list();
            }
            _ => self.other_token(),
        }
    }

    fn in_by_list(&self) -> bool {
        self.by_lists
            .get(self.calls.len())
            .copied()
            .unwrap_or(false)
    }
}

/// The unquoted value of the quoted text that starts at `start`, and the offset after it. A
/// doubled quote stands for one quote; an unterminated text runs to the end.
fn quoted(text: &str, start: usize, quote: char) -> (String, usize) {
    let mut value = String::new();
    let mut characters = text[start + 1..].char_indices().peekable();
    while let Some((offset, character)) = characters.next() {
        if character != quote {
            value.push(character);
        } else if characters.peek().is_some_and(|(_, next)| *next == quote) {
            value.push(quote);
            characters.next();
        } else {
            return (value, start + 1 + offset + 1);
        }
    }
    (value, text.len())
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || !byte.is_ascii()
}

fn word_end(bytes: &[u8], start: usize) -> usize {
    let mut end = start;
    while end < bytes.len() && is_word_byte(bytes[end]) {
        end += 1;
    }
    end
}

fn next_is_digit(bytes: &[u8], index: usize) -> bool {
    bytes.get(index + 1).is_some_and(u8::is_ascii_digit)
}

/// The offset after the number token that starts at `start`: digits, an optional fraction and
/// an optional exponent.
fn number_end(bytes: &[u8], start: usize) -> usize {
    let digits = |from: usize| {
        let mut end = from;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        end
    };
    let mut end = digits(start);
    if bytes.get(end) == Some(&b'.') {
        end = digits(end + 1);
    }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let sign = usize::from(matches!(bytes.get(end + 1), Some(b'+' | b'-')));
        if bytes.get(end + 1 + sign).is_some_and(u8::is_ascii_digit) {
            end = digits(end + 1 + sign);
        }
    }
    end
}

/// The digits of a number written `number`, without leading zeros of its integer part nor
/// trailing zeros of its fraction, so that `10.50`, `010.5` and `10.5` compare equal. A number
/// with an exponent is kept as written, upper-cased.
pub(crate) fn normalized_number(number: &str) -> String {
    let number = number.trim_start_matches(['+', '-']);
    if number.contains(['e', 'E']) {
        return number.to_ascii_uppercase();
    }
    let (integer, fraction) = number.split_once('.').unwrap_or((number, ""));
    let integer = integer.trim_start_matches('0');
    let fraction = fraction.trim_end_matches('0');
    let integer = if integer.is_empty() { "0" } else { integer };
    if fraction.is_empty() {
        integer.to_owned()
    } else {
        format!("{integer}.{fraction}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(text: &str) -> Vec<(LiteralKind, String)> {
        literals(text)
            .into_iter()
            .map(|literal| (literal.kind, literal.value))
            .collect()
    }

    #[test]
    fn finds_string_literals_and_number_tokens() {
        let found = values("sales.amount >= 10.50 AND sales.region = 'it''s' AND x < 3");
        assert_eq!(
            found,
            vec![
                (LiteralKind::Number, "10.5".to_owned()),
                (LiteralKind::Text, "it's".to_owned()),
                (LiteralKind::Number, "3".to_owned()),
            ]
        );
    }

    #[test]
    fn skips_identifiers_placeholders_and_their_digits() {
        let found = values("SELECT \"col 5\", col1, CAST($12 AS BIGINT) FROM t2");
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn gives_the_offset_and_the_preceding_word() {
        let found = literals("ORDER BY 1 LIMIT 10");
        let last = found.last().unwrap();
        assert_eq!(last.offset, 17);
        assert_eq!(last.end, 19);
        assert_eq!(last.preceding_word.as_deref(), Some("LIMIT"));
        assert_eq!(literals("ROUND(x, 2)")[0].preceding_word, None);
    }

    #[test]
    fn gives_the_function_and_the_argument() {
        let found = literals("SPLIT_PART(name, '-', 2) = (3)");
        assert_eq!(found[0].call, Some(("SPLIT_PART".to_owned(), 1)));
        assert_eq!(found[1].call, Some(("SPLIT_PART".to_owned(), 2)));
        assert_eq!(found[2].call, None);
    }

    #[test]
    fn marks_the_ordinals_of_by_lists() {
        let found = literals("SELECT a, 1 FROM t GROUP BY 1, 2 ORDER BY 2 DESC, a + 3 LIMIT 4");
        let ordinals: Vec<bool> = found.iter().map(|literal| literal.ordinal).collect();
        assert_eq!(ordinals, vec![false, true, true, true, false, false]);
    }

    #[test]
    fn normalizes_numbers() {
        assert_eq!(normalized_number("010.500"), "10.5");
        assert_eq!(normalized_number("0.0"), "0");
        assert_eq!(normalized_number(".5"), "0.5");
        assert_eq!(normalized_number("1e3"), "1E3");
    }
}

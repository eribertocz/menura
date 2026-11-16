//! The YAML 1.1 implicit resolution of plain scalars (E0A-2.2).
//!
//! The parser follows YAML 1.2, so the kind of an untagged plain scalar is decided here, with the
//! regular expressions of the YAML 1.1 type repository as the reference loader applies them, in
//! its order: boolean, float, integer, merge, null, timestamp, value. Anything else is a string.

use super::timestamp::is_implicit_timestamp;

/// What an untagged plain scalar is, before it is constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImplicitKind {
    Boolean(bool),
    Float,
    Integer,
    /// The merge key `<<`.
    Merge,
    Null,
    Timestamp,
    /// The YAML 1.1 value key `=`.
    Value,
    String,
}

/// Resolves an untagged plain scalar.
pub(crate) fn resolve(text: &str) -> ImplicitKind {
    if let Some(value) = boolean(text) {
        ImplicitKind::Boolean(value)
    } else if is_float(text) {
        ImplicitKind::Float
    } else if is_integer(text) {
        ImplicitKind::Integer
    } else if text == "<<" {
        ImplicitKind::Merge
    } else if matches!(text, "" | "~" | "null" | "Null" | "NULL") {
        ImplicitKind::Null
    } else if is_implicit_timestamp(text) {
        ImplicitKind::Timestamp
    } else if text == "=" {
        ImplicitKind::Value
    } else {
        ImplicitKind::String
    }
}

fn boolean(text: &str) -> Option<bool> {
    match text {
        "yes" | "Yes" | "YES" | "true" | "True" | "TRUE" | "on" | "On" | "ON" => Some(true),
        "no" | "No" | "NO" | "false" | "False" | "FALSE" | "off" | "Off" | "OFF" => Some(false),
        _ => None,
    }
}

/// A position in the bytes of a scalar, for the hand-written regular expressions below.
struct Cursor<'text> {
    bytes: &'text [u8],
    position: usize,
}

impl<'text> Cursor<'text> {
    fn new(text: &'text str) -> Self {
        Self {
            bytes: text.as_bytes(),
            position: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    /// Consumes `byte` if it is next.
    fn eat(&mut self, byte: u8) -> bool {
        self.eat_one(|next| next == byte)
    }

    /// Consumes the next byte if it satisfies `accept`.
    fn eat_one(&mut self, accept: impl Fn(u8) -> bool) -> bool {
        match self.peek() {
            Some(next) if accept(next) => {
                self.position += 1;
                true
            }
            _ => false,
        }
    }

    /// Consumes the longest run of bytes that satisfy `accept`, and whether it is not empty
    /// (`[...]+`).
    fn eat_some(&mut self, accept: impl Fn(u8) -> bool) -> bool {
        let start = self.position;
        while self.eat_one(&accept) {}
        self.position > start
    }

    /// Consumes the longest run of bytes that satisfy `accept`, possibly empty (`[...]*`); it
    /// always matches.
    fn eat_many(&mut self, accept: impl Fn(u8) -> bool) -> bool {
        self.eat_some(accept);
        true
    }

    /// Consumes `+` or `-` if it is next.
    fn eat_sign(&mut self) {
        self.eat_one(|next| next == b'+' || next == b'-');
    }

    /// Consumes `text` if it comes next.
    fn eat_text(&mut self, text: &str) -> bool {
        let matches = self
            .bytes
            .get(self.position..)
            .is_some_and(|rest| rest.starts_with(text.as_bytes()));
        if matches {
            self.position += text.len();
        }
        matches
    }

    fn at_end(&self) -> bool {
        self.position == self.bytes.len()
    }

    /// `(?::[0-5]?[0-9])+`, the sexagesimal tail of integers and floats.
    fn eat_sexagesimal_parts(&mut self) -> bool {
        let mut parts = 0;
        while self.eat(b':') {
            let two_digits = matches!(
                self.bytes.get(self.position..self.position + 2),
                Some([b'0'..=b'5', b'0'..=b'9'])
            );
            if two_digits {
                self.position += 2;
            } else if !self.eat_one(is_decimal) {
                return false;
            }
            parts += 1;
        }
        parts > 0
    }

    /// `(?:[eE][-+][0-9]+)?`
    fn eat_optional_exponent(&mut self) -> bool {
        if !self.eat_one(|next| next == b'e' || next == b'E') {
            return true;
        }
        self.eat_one(|next| next == b'+' || next == b'-') && self.eat_some(is_decimal)
    }
}

fn is_decimal(byte: u8) -> bool {
    byte.is_ascii_digit()
}

fn is_decimal_or_underscore(byte: u8) -> bool {
    byte.is_ascii_digit() || byte == b'_'
}

/// The YAML 1.1 integer expression.
fn is_integer(text: &str) -> bool {
    let alternatives: [fn(&mut Cursor<'_>) -> bool; 5] = [
        |cursor| {
            cursor.eat_sign();
            cursor.eat_text("0b") && cursor.eat_some(|next| matches!(next, b'0' | b'1' | b'_'))
        },
        |cursor| {
            cursor.eat_sign();
            cursor.eat(b'0') && cursor.eat_some(|next| matches!(next, b'0'..=b'7' | b'_'))
        },
        |cursor| {
            cursor.eat_sign();
            cursor.eat(b'0')
                || (cursor.eat_one(|next| matches!(next, b'1'..=b'9'))
                    && cursor.eat_many(is_decimal_or_underscore))
        },
        |cursor| {
            cursor.eat_sign();
            cursor.eat_text("0x")
                && cursor.eat_some(|next| next.is_ascii_hexdigit() || next == b'_')
        },
        |cursor| {
            cursor.eat_sign();
            cursor.eat_one(|next| matches!(next, b'1'..=b'9'))
                && cursor.eat_many(is_decimal_or_underscore)
                && cursor.eat_sexagesimal_parts()
        },
    ];
    matches_any(text, &alternatives)
}

/// The YAML 1.1 float expression.
fn is_float(text: &str) -> bool {
    let alternatives: [fn(&mut Cursor<'_>) -> bool; 5] = [
        |cursor| {
            cursor.eat_sign();
            cursor.eat_one(is_decimal)
                && cursor.eat_many(is_decimal_or_underscore)
                && cursor.eat(b'.')
                && cursor.eat_many(is_decimal_or_underscore)
                && cursor.eat_optional_exponent()
        },
        |cursor| {
            cursor.eat(b'.')
                && cursor.eat_one(is_decimal)
                && cursor.eat_many(is_decimal_or_underscore)
                && cursor.eat_optional_exponent()
        },
        |cursor| {
            cursor.eat_sign();
            cursor.eat_one(is_decimal)
                && cursor.eat_many(is_decimal_or_underscore)
                && cursor.eat_sexagesimal_parts()
                && cursor.eat(b'.')
                && cursor.eat_many(is_decimal_or_underscore)
        },
        |cursor| {
            cursor.eat_sign();
            cursor.eat_text(".inf") || cursor.eat_text(".Inf") || cursor.eat_text(".INF")
        },
        |cursor| cursor.eat_text(".nan") || cursor.eat_text(".NaN") || cursor.eat_text(".NAN"),
    ];
    matches_any(text, &alternatives)
}

/// Whether one alternative consumes the whole text.
fn matches_any(text: &str, alternatives: &[fn(&mut Cursor<'_>) -> bool]) -> bool {
    alternatives.iter().any(|alternative| {
        let mut cursor = Cursor::new(text);
        alternative(&mut cursor) && cursor.at_end()
    })
}

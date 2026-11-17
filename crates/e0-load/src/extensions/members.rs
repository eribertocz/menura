//! The member names of the JSON text of `data`, which a JSON reader does not keep (E0C-2.7).
//!
//! A JSON reader keeps one value of a repeated member name and drops the others silently. This
//! scanner walks text that the JSON reader has already accepted and reports what it drops: the
//! first repeated member name, and every value of the top-level `namespace` member.

use std::collections::BTreeSet;

use super::keys::NAMESPACE;

/// What the JSON reader does not keep of a valid JSON text.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Members<'text> {
    /// The first member name, in text order, repeated in one object anywhere in the text.
    pub(crate) repeated: Option<String>,
    /// The JSON text of every value of the top-level object's `namespace` member, in text order.
    pub(crate) namespaces: Vec<&'text str>,
}

/// Scans `text`, which MUST be valid JSON. On any other text the result is incomplete.
pub(crate) fn scan_members(text: &str) -> Members<'_> {
    let mut scanner = Scanner {
        text,
        position: 0,
        members: Members::default(),
    };
    scanner.value(true);
    scanner.members
}

struct Scanner<'text> {
    text: &'text str,
    /// A byte offset into `text`, always on a character boundary.
    position: usize,
    members: Members<'text>,
}

impl Scanner<'_> {
    fn peek(&self) -> Option<char> {
        self.text.get(self.position..)?.chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let next = self.peek()?;
        self.position += next.len_utf8();
        Some(next)
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(|next| " \t\n\r".contains(next)) {
            self.position += 1;
        }
    }

    /// Scans one value; `top_level` tells whether it is the whole text.
    fn value(&mut self, top_level: bool) {
        self.skip_whitespace();
        match self.peek() {
            Some('{') => self.object(top_level),
            Some('[') => self.array(),
            Some('"') => {
                self.string();
            }
            Some(_) => {
                while self.peek().is_some_and(|next| !",]} \t\n\r".contains(next)) {
                    self.advance();
                }
            }
            None => {}
        }
    }

    fn array(&mut self) {
        self.advance();
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some(']') | None => break,
                Some(',') => {
                    self.advance();
                }
                Some(_) => self.value(false),
            }
        }
        self.advance();
    }

    fn object(&mut self, top_level: bool) {
        self.advance();
        let mut names = BTreeSet::new();
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some('}') | None => break,
                Some(',') => {
                    self.advance();
                    continue;
                }
                Some(_) => {}
            }
            let name = self.string();
            self.skip_whitespace();
            self.advance();
            self.skip_whitespace();
            let start = self.position;
            if names.contains(&name) {
                self.members.repeated.get_or_insert_with(|| name.clone());
            }
            self.value(false);
            if top_level && name == NAMESPACE {
                let value = self.text.get(start..self.position).unwrap_or_default();
                self.members.namespaces.push(value);
            }
            names.insert(name);
        }
        self.advance();
    }

    /// Scans a string and returns its value, with its escapes decoded.
    fn string(&mut self) -> String {
        self.advance();
        let mut units: Vec<u16> = Vec::new();
        while let Some(next) = self.advance() {
            match next {
                '"' => break,
                '\\' => self.escape(&mut units),
                other => units.extend(other.encode_utf16(&mut [0; 2]).iter()),
            }
        }
        char::decode_utf16(units)
            .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect()
    }

    /// Decodes the escape after a backslash into UTF-16 code units, so that a surrogate pair
    /// written as two `\u` escapes decodes to one character.
    fn escape(&mut self, units: &mut Vec<u16>) {
        let decoded = match self.advance() {
            Some('u') => {
                let digits = self.text.get(self.position..self.position + 4);
                self.position += 4;
                digits.and_then(|digits| u16::from_str_radix(digits, 16).ok())
            }
            Some('b') => Some(0x08),
            Some('f') => Some(0x0C),
            Some('n') => Some(0x0A),
            Some('r') => Some(0x0D),
            Some('t') => Some(0x09),
            Some(other) => u16::try_from(u32::from(other)).ok(),
            None => None,
        };
        units.extend(decoded);
    }
}

#[cfg(test)]
mod tests {
    use super::{Members, scan_members};

    fn repeated(text: &str) -> Option<String> {
        scan_members(text).repeated
    }

    #[test]
    fn e0c_2_7_repeated_names_are_found_at_any_depth() {
        assert_eq!(repeated(r#"{"a": 1, "b": 2}"#), None);
        assert_eq!(repeated(r#"{"a": 1, "a": 2}"#).as_deref(), Some("a"));
        assert_eq!(
            repeated(r#"[1, "x", {"o": {"p": [true, null], "p": -1.5e3}}]"#).as_deref(),
            Some("p")
        );
        assert_eq!(
            repeated(r#"{"a": {"a": 1}, "b": [{"a": 1}, {"a": 2}]}"#),
            None
        );
        assert_eq!(
            repeated(r#"{"z": 0, "z": {"y": 1, "y": 2}}"#).as_deref(),
            Some("z")
        );
        assert_eq!(
            repeated(r#"{"z": {"y": 1, "y": 2}, "z": 0}"#).as_deref(),
            Some("y")
        );
    }

    #[test]
    fn e0c_2_7_names_are_compared_after_decoding_escapes() {
        assert_eq!(repeated(r#"{"a": 1, "a": 2}"#).as_deref(), Some("a"));
        assert_eq!(repeated(r#"{"😀": 1, "😀": 2}"#).as_deref(), Some("😀"));
        assert_eq!(
            repeated(r#"{"q\"\\\/\b\f\n\r\t": 1, "q\"\\/\b\f\n\r\t": 2}"#).as_deref(),
            Some("q\"\\/\u{8}\u{c}\n\r\t")
        );
        assert_eq!(repeated(r#"{"a]}": "}{,", "b": "a"}"#), None);
    }

    #[test]
    fn every_top_level_namespace_value_is_kept() {
        let text = r#"{"namespace": "acme", "x": {"namespace": "inner"}, "namespace" : "semantic_compiler" }"#;
        assert_eq!(
            scan_members(text),
            Members {
                repeated: Some("namespace".to_owned()),
                namespaces: vec![r#""acme""#, r#""semantic_compiler""#],
            }
        );
    }
}

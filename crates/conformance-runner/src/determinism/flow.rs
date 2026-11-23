//! Reorders the mappings of a JSON document or a YAML flow collection, recursively, keeping the
//! text of every scalar, a string that holds a JSON document included, and the order of every
//! sequence.

use super::permutation::Permutation;

/// `text`, a JSON document or a YAML flow value, with the entries of every mapping reordered,
/// or `None` when it is not a flow value this reader understands (comments, multi-line plain
/// scalars, keys without values): the caller then keeps the text unchanged.
pub(crate) fn reorder_flow(text: &str, permutation: &mut Permutation) -> Option<String> {
    let mut reader = Reader { text, position: 0 };
    let value = reader.value(permutation)?;
    reader.skip_spaces()?;
    reader.peek().is_none().then_some(value)
}

/// A cursor over the text of one flow value.
struct Reader<'text> {
    text: &'text str,
    position: usize,
}

impl Reader<'_> {
    fn rest(&self) -> &str {
        self.text.get(self.position..).unwrap_or_default()
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn advance(&mut self, character: char) {
        self.position += character.len_utf8();
    }

    /// Skips white space; `None` at a comment, which this reader does not keep.
    fn skip_spaces(&mut self) -> Option<()> {
        while let Some(character) = self.peek() {
            match character {
                ' ' | '\t' | '\n' | '\r' => self.advance(character),
                '#' => return None,
                _ => break,
            }
        }
        Some(())
    }

    /// The expected punctuation `character`, after white space.
    fn expect(&mut self, character: char) -> Option<()> {
        self.skip_spaces()?;
        (self.peek() == Some(character)).then(|| self.advance(character))
    }

    fn value(&mut self, permutation: &mut Permutation) -> Option<String> {
        self.skip_spaces()?;
        match self.peek()? {
            '{' => self.mapping(permutation),
            '[' => self.sequence(permutation),
            _ => self.scalar(),
        }
    }

    fn mapping(&mut self, permutation: &mut Permutation) -> Option<String> {
        self.advance('{');
        let entries = self.items('}', |reader| {
            let key = reader.scalar()?;
            reader.expect(':')?;
            let value = reader.value(permutation)?;
            Some(format!("{key}: {value}"))
        })?;
        Some(format!("{{{}}}", permutation.reorder(entries).join(", ")))
    }

    fn sequence(&mut self, permutation: &mut Permutation) -> Option<String> {
        self.advance('[');
        let items = self.items(']', |reader| reader.value(permutation))?;
        Some(format!("[{}]", items.join(", ")))
    }

    /// The items up to `close`, separated by commas, with an optional final comma.
    fn items(
        &mut self,
        close: char,
        mut item: impl FnMut(&mut Self) -> Option<String>,
    ) -> Option<Vec<String>> {
        let mut items = Vec::new();
        loop {
            self.skip_spaces()?;
            if self.peek()? == close {
                self.advance(close);
                return Some(items);
            }
            items.push(item(self)?);
            self.skip_spaces()?;
            match self.peek()? {
                ',' => self.advance(','),
                found if found == close => {}
                _ => return None,
            }
        }
    }

    /// The text of a scalar, quotes included.
    fn scalar(&mut self) -> Option<String> {
        self.skip_spaces()?;
        let start = self.position;
        match self.peek()? {
            '"' => self.double_quoted()?,
            '\'' => self.single_quoted()?,
            '{' | '}' | '[' | ']' | ',' => return None,
            _ => self.plain()?,
        }
        self.text.get(start..self.position).map(str::to_owned)
    }

    fn double_quoted(&mut self) -> Option<()> {
        self.advance('"');
        let mut escaped = false;
        while let Some(character) = self.peek() {
            self.advance(character);
            match (escaped, character) {
                (false, '"') => return Some(()),
                (false, '\\') => escaped = true,
                _ => escaped = false,
            }
        }
        None
    }

    fn single_quoted(&mut self) -> Option<()> {
        self.advance('\'');
        while let Some(character) = self.peek() {
            self.advance(character);
            if character == '\'' {
                if self.peek() == Some('\'') {
                    self.advance('\'');
                } else {
                    return Some(());
                }
            }
        }
        None
    }

    /// A plain scalar up to a flow indicator, `: ` or the end of its line; trailing spaces are
    /// not part of it. A plain scalar continued on the next line leaves its collection unread.
    fn plain(&mut self) -> Option<()> {
        let start = self.position;
        let mut end = start;
        while let Some(character) = self.peek() {
            let after = self.rest().chars().nth(1);
            let ends_key = character == ':'
                && after.is_none_or(|next| matches!(next, ' ' | ',' | ']' | '}' | '\n'));
            if matches!(character, ',' | '[' | ']' | '{' | '}' | '\n' | '\r') || ends_key {
                break;
            }
            if character == '#' && self.rest_before_is_space() {
                return None;
            }
            self.advance(character);
            if character != ' ' && character != '\t' {
                end = self.position;
            }
        }
        (end > start).then(|| self.position = end)
    }

    fn rest_before_is_space(&self) -> bool {
        self.text
            .get(..self.position)
            .is_some_and(|before| before.ends_with([' ', '\t']))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::determinism::permutation::SEED;

    fn reorder(text: &str) -> Option<String> {
        reorder_flow(text, &mut Permutation::new(SEED))
    }

    #[test]
    fn pln_2_4_mappings_are_reordered_and_sequences_kept() {
        let text = "{\"a\": 1, \"b\": [3, 2, {\"c\": true, \"d\": null}], \"e\": \"x, y\"}";
        let reordered = reorder(text).unwrap();
        assert_ne!(reordered, text);
        let read = |text: &str| serde_json::from_str::<serde_json::Value>(text).unwrap();
        assert_eq!(read(text), read(&reordered));
        assert!(reordered.contains("[3, 2, "));
    }

    #[test]
    fn pln_2_4_yaml_flow_scalars_keep_their_text() {
        let text = "{dialects: [{dialect: ANSI_SQL, expression: 'CAST(a AS DECIMAL(10, 0))'}]}";
        let reordered = reorder(text).unwrap();
        assert!(reordered.contains("dialect: ANSI_SQL"));
        assert!(reordered.contains("expression: 'CAST(a AS DECIMAL(10, 0))'"));
    }

    #[test]
    fn pln_2_4_unknown_text_is_not_reordered() {
        assert_eq!(reorder("{a: 1 # comment\n}"), None);
        assert_eq!(reorder("{a: 1"), None);
        assert_eq!(reorder("{a}"), None);
    }
}

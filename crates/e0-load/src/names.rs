//! Identifier normalization and comparison (E0D-1, CON-5), shared by the substages that compare
//! names.

use menura_contracts::normalize_unquoted_identifier;
use menura_model::expression::Identifier;

/// The most characters an identifier may have (E0D-1.2).
pub(crate) const MAXIMUM_LENGTH: usize = 128;

/// An identifier after normalization (E0D-1.1): two names are the same name exactly when their
/// normalized forms are equal (E0D-1.3).
///
/// A regular identifier is normalized to upper case; a delimited one is kept exactly, without
/// its quotes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Name(String);

impl Name {
    /// The name declared by a string of the model document (E0D-1.4): a delimited identifier
    /// when the string starts and ends with `"`, otherwise a regular identifier.
    pub(crate) fn declared(text: &str) -> Self {
        match delimited_content(text) {
            Some(content) => Self(content),
            None => Self::regular(text),
        }
    }

    /// The name of a regular identifier spelled `text`, such as a catalog column name compared
    /// with a regular reference (E0D-1.5).
    pub(crate) fn regular(text: &str) -> Self {
        Self(normalize_unquoted_identifier(text))
    }

    /// The name an identifier of an expression refers to (E0D-1.1).
    pub(crate) fn of(identifier: &Identifier) -> Self {
        if identifier.is_delimited() {
            Self(identifier.text().to_owned())
        } else {
            Self::regular(identifier.text())
        }
    }

    /// The normalized text.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether the declared name `text` is a delimited identifier (E0D-1.4).
pub(crate) fn is_delimited(text: &str) -> bool {
    text.len() >= 2 && text.starts_with('"') && text.ends_with('"')
}

/// The identifier text of the declared name `text`: without its quotes and with `""` read as
/// `"` when it is delimited, otherwise `text` itself.
pub(crate) fn identifier_text(text: &str) -> String {
    delimited_content(text).unwrap_or_else(|| text.to_owned())
}

/// Whether the declared name `text` is a malformed delimited name (E0D-1.4): it starts with
/// `"` but is not a delimited identifier, because it does not end with `"`, has no character
/// between its quotes, or has a `"` inside them that is not doubled.
///
/// `"a"b"`, `"`, `""` and `"a` are malformed; `"a"`, `"a""b"` and `a"b` are not.
pub(crate) fn is_malformed(text: &str) -> bool {
    let Some(rest) = text.strip_prefix('"') else {
        return false;
    };
    match rest.strip_suffix('"') {
        Some(inner) => inner.is_empty() || !quotes_are_doubled(inner),
        None => true,
    }
}

/// Whether every `"` of `inner`, the text between the quotes of a delimited name, is part of a
/// `""` pair.
fn quotes_are_doubled(inner: &str) -> bool {
    let mut characters = inner.chars();
    while let Some(character) = characters.next() {
        if character == '"' && characters.next() != Some('"') {
            return false;
        }
    }
    true
}

fn delimited_content(text: &str) -> Option<String> {
    let inner = text.strip_prefix('"')?.strip_suffix('"')?;
    is_delimited(text).then(|| inner.replace("\"\"", "\""))
}

/// Whether the identifier text `text` has more characters than an identifier may have
/// (E0D-1.2). Characters are Unicode scalar values; the quotes of a delimited identifier do not
/// count.
pub(crate) fn is_too_long(text: &str) -> bool {
    text.chars().nth(MAXIMUM_LENGTH).is_some()
}

#[cfg(test)]
mod tests {
    use menura_model::expression::Identifier;

    use super::{MAXIMUM_LENGTH, Name, identifier_text, is_delimited, is_malformed, is_too_long};

    fn regular(text: &str) -> Identifier {
        Identifier::regular(text).expect("a regular identifier")
    }

    fn delimited(text: &str) -> Identifier {
        Identifier::delimited(text).expect("a delimited identifier")
    }

    #[test]
    fn e0d_1_1_unquoted_names_are_upper_case_and_quoted_names_exact() {
        assert_eq!(Name::of(&regular("Orders")), Name::declared("orders"));
        assert_eq!(Name::of(&delimited("ORDERS")), Name::declared("orders"));
        assert_ne!(Name::of(&delimited("orders")), Name::declared("orders"));
        assert_ne!(Name::of(&delimited("Orders")), Name::of(&regular("Orders")));
    }

    #[test]
    fn e0d_1_4_a_declared_name_between_double_quotes_is_delimited() {
        assert_eq!(Name::declared("\"Orders\""), Name::of(&delimited("Orders")));
        assert_ne!(Name::declared("\"Orders\""), Name::declared("orders"));
        assert_eq!(Name::declared("\"ORDERS\""), Name::declared("orders"));
        assert_eq!(Name::declared("\"a\"\"b\""), Name::of(&delimited("a\"b")));
        assert_eq!(identifier_text("\"a\"\"b\""), "a\"b");
        assert!(is_delimited("\"\""));
        assert!(!is_delimited("\""));
        assert!(!is_delimited("\"Orders"));
        assert_eq!(Name::declared("\"Orders"), Name::regular("\"ORDERS"));
    }

    #[test]
    fn e0d_1_1_normalization_is_the_simple_mapping() {
        assert_eq!(Name::declared("straße"), Name::of(&delimited("STRAßE")));
        assert_eq!(Name::declared("ǆ"), Name::of(&delimited("Ǆ")));
        assert_eq!(Name::declared("\u{1FB3}"), Name::of(&delimited("\u{1FBC}")));
        assert_eq!(Name::declared("\u{1F80}"), Name::of(&delimited("\u{1F88}")));
        assert_eq!(Name::declared("\u{1F88}"), Name::of(&delimited("\u{1F88}")));
        assert_eq!(Name::declared("ŉ"), Name::of(&delimited("ŉ")));
    }

    #[test]
    fn e0d_1_2_at_most_128_characters() {
        assert!(!is_too_long(&"é".repeat(MAXIMUM_LENGTH)));
        assert!(is_too_long(&"a".repeat(MAXIMUM_LENGTH + 1)));
        let quoted = format!("\"{}\"", "a".repeat(MAXIMUM_LENGTH));
        assert!(!is_too_long(&identifier_text(&quoted)));
    }

    #[test]
    fn e0d_1_4_e0d_01_a_quoted_name_that_is_not_delimited_is_malformed() {
        for text in ["\"a\"b\"", "\"", "\"\"", "\"a", "\"\"\"", "\"a\"\"\"b\""] {
            assert!(is_malformed(text), "{text} is malformed");
        }
        for text in [
            "\"a\"",
            "\"a\"\"b\"",
            "\"\"\"\"",
            "a\"",
            "a\"b",
            "orders",
            "",
        ] {
            assert!(!is_malformed(text), "{text} is not malformed");
        }
    }
}

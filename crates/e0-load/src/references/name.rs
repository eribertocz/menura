//! Identifier normalization and comparison (E0D-1, CON-5).

use semantic_compiler_model::expression::Identifier;

/// The most characters an identifier may have (E0D-1.2).
pub(crate) const MAXIMUM_LENGTH: usize = 128;

/// An identifier after normalization (E0D-1.1): two names are the same name exactly when their
/// normalized forms are equal (E0D-1.3).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Name(String);

impl Name {
    /// The name declared by a string of the model document or the catalog schema.
    ///
    /// A declared name is an unquoted identifier: names that differ only in case are the same
    /// name (DEV-2). Whether a declared name may be written as a delimited identifier is open
    /// (issue #78).
    pub(crate) fn declared(text: &str) -> Self {
        Self(upper_case(text))
    }

    /// The name an identifier of an expression refers to: a regular identifier in upper case,
    /// a delimited one exactly as written (E0D-1.1).
    pub(crate) fn of(identifier: &Identifier) -> Self {
        if identifier.is_delimited() {
            Self(identifier.text().to_owned())
        } else {
            Self::declared(identifier.text())
        }
    }
}

/// Whether `text` has more characters than an identifier may have (E0D-1.2). Characters are
/// Unicode scalar values.
pub(crate) fn is_too_long(text: &str) -> bool {
    text.chars().nth(MAXIMUM_LENGTH).is_some()
}

/// `text` with every character replaced by its Unicode simple upper-case mapping, so the length
/// never changes (CON-5).
fn upper_case(text: &str) -> String {
    text.chars().map(simple_upper_case).collect()
}

/// The simple upper-case mapping of one character.
///
/// The standard library gives the full mapping. Where it is one character, it is the simple
/// mapping. Where it is several, the simple mapping is the character itself, except for the
/// Greek letters with ypogegrammeni, whose simple mapping is the letter with prosgegrammeni.
fn simple_upper_case(character: char) -> char {
    let mut full = character.to_uppercase();
    match (full.next(), full.next()) {
        (Some(single), None) => single,
        _ => with_prosgegrammeni(character).unwrap_or(character),
    }
}

/// The simple upper-case mapping of a lower-case Greek letter with ypogegrammeni.
fn with_prosgegrammeni(character: char) -> Option<char> {
    let code = u32::from(character);
    let upper = match code {
        0x1F80..=0x1F87 | 0x1F90..=0x1F97 | 0x1FA0..=0x1FA7 => code + 8,
        0x1FB3 | 0x1FC3 | 0x1FF3 => code + 9,
        _ => return None,
    };
    char::from_u32(upper)
}

#[cfg(test)]
mod tests {
    use semantic_compiler_model::expression::Identifier;

    use super::{MAXIMUM_LENGTH, Name, is_too_long};

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
    }
}

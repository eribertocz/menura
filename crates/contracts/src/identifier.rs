//! Normalization of an unquoted identifier (E0D-1.1) with the Unicode simple upper-case mapping
//! (CON-5).

/// The normalized form of `name` read as an unquoted identifier.
///
/// Each character is replaced by its Unicode simple upper-case mapping, so the number of
/// characters never changes (CON-5).
pub(crate) fn normalize_unquoted(name: &str) -> String {
    name.chars().map(simple_upper_case).collect()
}

/// The Unicode simple upper-case mapping of one character.
///
/// `char::to_uppercase` is the full mapping. Where the full mapping has one character it is the
/// simple mapping. Where it has several, the simple mapping of the Unicode Character Database is
/// the character itself, except for the Greek letters with a small iota subscript handled by
/// [`iota_subscript_upper_case`].
fn simple_upper_case(character: char) -> char {
    let mut full = character.to_uppercase();
    match (full.next(), full.next()) {
        (Some(single), None) => single,
        _ => iota_subscript_upper_case(character).unwrap_or(character),
    }
}

/// The simple upper-case mapping of a lower-case Greek letter with a small iota subscript
/// (`UnicodeData.txt`, field 12), whose full mapping has two characters.
fn iota_subscript_upper_case(character: char) -> Option<char> {
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
    use super::normalize_unquoted;

    #[test]
    fn e0d_1_1_letters_are_upper_case() {
        assert_eq!(normalize_unquoted("amount_2"), "AMOUNT_2");
        assert_eq!(normalize_unquoted("r\u{e9}gion"), "R\u{c9}GION");
        assert_eq!(normalize_unquoted("\u{1c5}"), "\u{1c4}");
    }

    #[test]
    fn con_5_the_simple_mapping_keeps_the_number_of_characters() {
        assert_eq!(normalize_unquoted("stra\u{df}e"), "STRA\u{df}E");
        assert_eq!(normalize_unquoted("\u{fb01}"), "\u{fb01}");
        assert_eq!(normalize_unquoted("\u{149}"), "\u{149}");
        assert_eq!(normalize_unquoted("\u{1f88}"), "\u{1f88}");
    }

    #[test]
    fn con_5_iota_subscript_letters_map_to_their_title_case_form() {
        assert_eq!(normalize_unquoted("\u{1f80}"), "\u{1f88}");
        assert_eq!(normalize_unquoted("\u{1f97}"), "\u{1f9f}");
        assert_eq!(normalize_unquoted("\u{1fa7}"), "\u{1faf}");
        assert_eq!(normalize_unquoted("\u{1fb3}"), "\u{1fbc}");
        assert_eq!(normalize_unquoted("\u{1fc3}"), "\u{1fcc}");
        assert_eq!(normalize_unquoted("\u{1ff3}"), "\u{1ffc}");
        assert_eq!(normalize_unquoted("\u{1fb2}"), "\u{1fb2}");
    }
}

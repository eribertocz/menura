//! A declared name read as an identifier (E0D-1.4) and normalized (E0D-1.1), so that two names
//! are the same name exactly when their normalized forms are equal (E0D-1.3).
//!
//! E0B-3.2 reads the load option `model_name` this way and matches it with the semantic model
//! `name`.

/// The normalized form of the declared name `text` (E0D-1.4, E0D-1.1).
///
/// A delimited identifier (text that starts and ends with `"`, with `""` standing for `"`) is
/// kept exactly, without its quotes. Any other text is a regular identifier and is normalized to
/// upper case with the Unicode simple upper-case mapping, so its length never changes (CON-5).
pub(crate) fn normalized_name(text: &str) -> String {
    match delimited_content(text) {
        Some(content) => content,
        None => text.chars().map(simple_upper_case).collect(),
    }
}

/// The content of a delimited identifier, or nothing for a regular one.
fn delimited_content(text: &str) -> Option<String> {
    if text.len() < 2 {
        return None;
    }
    let inner = text.strip_prefix('"')?.strip_suffix('"')?;
    Some(inner.replace("\"\"", "\""))
}

/// The Unicode simple upper-case mapping of one character.
///
/// `char::to_uppercase` is the full mapping. Where it is one character, it is the simple
/// mapping. Where it is several, the simple mapping is the character itself, except for the
/// lower-case Greek letters with a small iota subscript (`UnicodeData.txt`, field 12).
fn simple_upper_case(character: char) -> char {
    let mut full = character.to_uppercase();
    match (full.next(), full.next()) {
        (Some(single), None) => single,
        _ => iota_subscript_upper_case(character).unwrap_or(character),
    }
}

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
    use super::normalized_name;

    #[test]
    fn e0d_1_1_a_regular_identifier_is_upper_case() {
        assert_eq!(normalized_name("shop_2"), "SHOP_2");
        assert_eq!(normalized_name("r\u{e9}gion"), "R\u{c9}GION");
    }

    #[test]
    fn e0d_1_4_a_delimited_identifier_is_compared_exactly_without_quotes() {
        assert_eq!(normalized_name("\"Shop\""), "Shop");
        assert_eq!(normalized_name("\"say \"\"hi\"\"\""), "say \"hi\"");
        assert_eq!(normalized_name("\""), "\"");
    }

    #[test]
    fn con_5_the_simple_mapping_keeps_the_number_of_characters() {
        assert_eq!(normalized_name("stra\u{df}e"), "STRA\u{df}E");
        assert_eq!(normalized_name("\u{fb01}"), "\u{fb01}");
        assert_eq!(normalized_name("\u{1f80}"), "\u{1f88}");
        assert_eq!(normalized_name("\u{1fb3}"), "\u{1fbc}");
    }
}

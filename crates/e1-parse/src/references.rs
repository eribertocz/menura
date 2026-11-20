//! Wide-table references, split into parts as E1-3.1 and E0D-1.4 read them.

use menura_representations::query_tree::{QueryReference, ReferencePart};

/// The double quote that delimits an identifier.
const QUOTE: char = '"';

/// Splits `text` into parts at each `.` outside a delimited identifier. E1 never rejects a
/// reference: an empty or malformed part is kept for E2 to report (E1-3.1).
///
/// A split always yields at least one part, so the result is `None` only if that invariant is
/// broken.
pub(crate) fn split_reference(text: &str) -> Option<QueryReference> {
    QueryReference::new(part_texts(text).into_iter().map(read_part).collect())
}

/// The texts between the `.` that are outside double quotes.
fn part_texts(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut inside_quotes = false;
    let mut start = 0;
    for (offset, character) in text.char_indices() {
        if character == QUOTE {
            inside_quotes = !inside_quotes;
        } else if character == '.' && !inside_quotes {
            parts.push(&text[start..offset]);
            start = offset + 1;
        }
    }
    parts.push(&text[start..]);
    parts
}

/// Reads one part: delimited when it starts and ends with `"`, regular otherwise; a part that
/// starts with `"` and is not a well-formed delimited identifier is malformed (E0D-1.4).
fn read_part(text: &str) -> ReferencePart {
    if !text.starts_with(QUOTE) {
        return ReferencePart::Regular(text.to_owned());
    }
    match delimited_content(text) {
        Some(content) => ReferencePart::Delimited(content),
        None => ReferencePart::Malformed(text.to_owned()),
    }
}

/// The content of a delimited identifier, with `""` read as `"`. `None` when the text does not
/// end with `"`, has no character between its quotes, or has a `"` that is not doubled.
fn delimited_content(text: &str) -> Option<String> {
    let inner = text.strip_prefix(QUOTE)?.strip_suffix(QUOTE)?;
    if inner.is_empty() {
        return None;
    }
    let mut content = String::with_capacity(inner.len());
    let mut characters = inner.chars();
    while let Some(character) = characters.next() {
        if character == QUOTE && characters.next() != Some(QUOTE) {
            return None;
        }
        content.push(character);
    }
    Some(content)
}

#[cfg(test)]
mod tests {
    use menura_representations::query_tree::ReferencePart;

    use super::read_part;

    #[test]
    fn e1_3_1_reads_parts_as_e0d_1_4() {
        let delimited = |text: &str| ReferencePart::Delimited(text.to_owned());
        let malformed = |text: &str| ReferencePart::Malformed(text.to_owned());
        assert_eq!(
            read_part("Region"),
            ReferencePart::Regular("Region".to_owned())
        );
        assert_eq!(read_part("\"a\"\"b\""), delimited("a\"b"));
        assert_eq!(read_part("\""), malformed("\""));
        assert_eq!(read_part("\"\""), malformed("\"\""));
        assert_eq!(read_part("\"a"), malformed("\"a"));
        assert_eq!(read_part("\"a\"b\""), malformed("\"a\"b\""));
        assert_eq!(read_part("\"a\"\""), malformed("\"a\"\""));
    }
}

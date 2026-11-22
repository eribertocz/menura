//! The top-level members of the JSON text of a release-format `data`, with the byte range of
//! each value token, so a rewrite can replace one token and keep every other character
//! (MIG-3.2a).

use std::ops::Range;

/// One top-level member of a JSON object text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TextMember {
    /// The member name, unescaped.
    pub(super) name: String,
    /// The byte range of its value token in the text.
    pub(super) value: Range<usize>,
}

/// The top-level members of `text`, a valid JSON object text, in text order, repeated names
/// included; `None` when `text` is not one.
pub(super) fn top_level_members(text: &str) -> Option<Vec<TextMember>> {
    let bytes = text.as_bytes();
    let mut position = skip_whitespace(bytes, 0);
    if bytes.get(position) != Some(&b'{') {
        return None;
    }
    position = skip_whitespace(bytes, position + 1);
    let mut members = Vec::new();
    if bytes.get(position) == Some(&b'}') {
        return Some(members);
    }
    loop {
        let name_end = string_end(bytes, position)?;
        let name: String = serde_json::from_str(text.get(position..name_end)?).ok()?;
        position = skip_whitespace(bytes, name_end);
        if bytes.get(position) != Some(&b':') {
            return None;
        }
        let value_start = skip_whitespace(bytes, position + 1);
        let value_end = value_end(bytes, value_start)?;
        members.push(TextMember {
            name,
            value: value_start..value_end,
        });
        position = skip_whitespace(bytes, value_end);
        match bytes.get(position) {
            Some(b',') => position = skip_whitespace(bytes, position + 1),
            Some(b'}') => return Some(members),
            _ => return None,
        }
    }
}

/// The first position at or after `position` that is not JSON whitespace.
fn skip_whitespace(bytes: &[u8], mut position: usize) -> usize {
    while matches!(bytes.get(position), Some(b' ' | b'\t' | b'\n' | b'\r')) {
        position += 1;
    }
    position
}

/// The end of the string token that starts at `position`.
fn string_end(bytes: &[u8], position: usize) -> Option<usize> {
    if bytes.get(position) != Some(&b'"') {
        return None;
    }
    let mut index = position + 1;
    loop {
        match bytes.get(index)? {
            b'\\' => index += 2,
            b'"' => return Some(index + 1),
            _ => index += 1,
        }
    }
}

/// The end of the value token that starts at `position`.
fn value_end(bytes: &[u8], position: usize) -> Option<usize> {
    match bytes.get(position)? {
        b'"' => string_end(bytes, position),
        b'{' | b'[' => container_end(bytes, position),
        _ => {
            let mut index = position;
            while bytes.get(index).is_some_and(|byte| {
                !matches!(byte, b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r')
            }) {
                index += 1;
            }
            (index > position).then_some(index)
        }
    }
}

/// The end of the object or array token that starts at `position`.
fn container_end(bytes: &[u8], position: usize) -> Option<usize> {
    let mut depth = 0_usize;
    let mut index = position;
    loop {
        match bytes.get(index)? {
            b'"' => {
                index = string_end(bytes, index)?;
                continue;
            }
            b'{' | b'[' => depth += 1,
            b'}' | b']' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index + 1);
                }
            }
            _ => {}
        }
        index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn members_carry_the_range_of_their_value() {
        let text = r#"{"a": {"b": ["}", 1]}, "a\"": -1.5e3 , "c":"x"}"#;
        let members = top_level_members(text).unwrap();
        let found: Vec<(&str, &str)> = members
            .iter()
            .map(|member| (member.name.as_str(), &text[member.value.clone()]))
            .collect();
        assert_eq!(
            found,
            [
                ("a", r#"{"b": ["}", 1]}"#),
                ("a\"", "-1.5e3"),
                ("c", r#""x""#)
            ]
        );
    }

    #[test]
    fn an_empty_object_has_no_member_and_other_texts_none() {
        assert_eq!(top_level_members(" {} "), Some(Vec::new()));
        assert_eq!(top_level_members("[]"), None);
        assert_eq!(top_level_members(r#"{"a" 1}"#), None);
    }
}

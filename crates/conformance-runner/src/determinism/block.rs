//! Reorders the entries of every mapping of a YAML document in block style, recursively,
//! moving whole lines, so that every scalar keeps its text and every sequence its order.
//! Flow collections on one line are reordered by [`super::flow`].

use super::flow::{reorder_embedded, reorder_flow};
use super::permutation::Permutation;

/// The lines of `text` with the entries of every mapping reordered, or `None` when the
/// document uses a form this reader does not move (document markers, directives, tabs in an
/// indentation): the caller then keeps the text unchanged.
pub(crate) fn reorder_block(text: &str, permutation: &mut Permutation) -> Option<String> {
    let lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let marked = lines
        .iter()
        .any(|line| line.starts_with("---") || line.starts_with("...") || line.starts_with('%'));
    if marked {
        return None;
    }
    let reordered = reorder_node(&lines, permutation)?;
    let ending = if text.ends_with('\n') { "\n" } else { "" };
    Some(format!("{}{ending}", reordered.join("\n")))
}

/// Whether a line holds no content: blank, or a comment.
fn is_blank(line: &str) -> bool {
    let content = line.trim_start_matches(' ');
    content.is_empty() || content.starts_with('#')
}

/// The indentation of a line; `None` when it starts with a tab.
fn indentation(line: &str) -> Option<usize> {
    let spaces = line.len() - line.trim_start_matches(' ').len();
    (!line.get(spaces..)?.starts_with('\t')).then_some(spaces)
}

/// Whether the content of a line, after its indentation, starts a sequence item.
fn is_item(content: &str) -> bool {
    content == "-" || content.starts_with("- ")
}

/// The lines of one node: a mapping, a sequence, or a single-line value.
fn reorder_node(lines: &[String], permutation: &mut Permutation) -> Option<Vec<String>> {
    let Some(first) = lines.iter().position(|line| !is_blank(line)) else {
        return Some(lines.to_vec());
    };
    let (leading, body) = lines.split_at(first);
    let indent = indentation(body.first()?)?;
    let content = body.first()?.get(indent..)?;
    let mut result = leading.to_vec();
    if is_item(content) {
        for group in groups(body, indent, true)? {
            result.extend(reorder_item(group, indent, permutation)?);
        }
    } else if split_key(content).is_some() {
        let mut entries = Vec::new();
        for group in groups(body, indent, false)? {
            entries.push(reorder_entry(group, indent, permutation)?);
        }
        result.extend(permutation.reorder(entries).into_iter().flatten());
    } else {
        result.extend(reorder_value_lines(body, indent, permutation));
    }
    Some(result)
}

/// The groups of lines that each start an entry or an item at `indent`. In a mapping, an item
/// at the same indentation belongs to the entry before it.
fn groups(lines: &[String], indent: usize, items: bool) -> Option<Vec<&[String]>> {
    let mut starts = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if is_blank(line) {
            continue;
        }
        let line_indent = indentation(line)?;
        if line_indent < indent {
            return None;
        }
        let content = line.get(line_indent..)?;
        if line_indent == indent && (items || !is_item(content)) {
            starts.push(index);
        }
    }
    let mut groups = Vec::new();
    for (position, start) in starts.iter().enumerate() {
        let end = starts.get(position + 1).copied().unwrap_or(lines.len());
        groups.push(lines.get(*start..end)?);
    }
    Some(groups)
}

/// The key of a mapping entry and the rest of its line, after `:`; `None` when the content is
/// not `key:` or `key: value` with a plain or quoted key.
fn split_key(content: &str) -> Option<(&str, &str)> {
    let mut quote: Option<char> = None;
    let mut previous = ' ';
    for (index, character) in content.char_indices() {
        match (quote, character) {
            (None, '{' | '[' | '?' | '&' | '*' | '!' | '|' | '>' | '#') if index == 0 => {
                return None;
            }
            (None, '"' | '\'') if index == 0 => quote = Some(character),
            (None, '#') if previous == ' ' => return None,
            (None, ':') => {
                let rest = content.get(index + 1..)?;
                if rest.is_empty() || rest.starts_with(' ') {
                    return Some((content.get(..index)?, rest));
                }
            }
            (Some(open), found) if open == found => quote = None,
            _ => {}
        }
        previous = character;
    }
    None
}

/// One mapping entry: its first line with its value reordered, then its other lines.
fn reorder_entry(
    group: &[String],
    indent: usize,
    permutation: &mut Permutation,
) -> Option<Vec<String>> {
    let (first, nested) = group.split_first()?;
    let (key, rest) = split_key(first.get(indent..)?)?;
    let value = rest.trim_start();
    let prefix = " ".repeat(indent);
    if value.is_empty() || value.starts_with('#') {
        let mut lines = vec![first.clone()];
        lines.extend(reorder_node(nested, permutation)?);
        return Some(lines);
    }
    let has_more = nested.iter().any(|line| !is_blank(line));
    if has_more || is_left_unchanged(value) {
        return Some(group.to_vec());
    }
    let mut lines = vec![format!(
        "{prefix}{key}: {}",
        reorder_value(value, permutation)
    )];
    lines.extend(nested.iter().cloned());
    Some(lines)
}

/// One sequence item, with its content reordered as a node indented past its dash.
fn reorder_item(
    group: &[String],
    indent: usize,
    permutation: &mut Permutation,
) -> Option<Vec<String>> {
    let (first, nested) = group.split_first()?;
    let after_dash = first.get(indent + 1..)?;
    let value = after_dash.trim_start_matches(' ');
    if value.is_empty() || value.starts_with('#') {
        let mut lines = vec![first.clone()];
        lines.extend(reorder_node(nested, permutation)?);
        return Some(lines);
    }
    if is_left_unchanged(value) {
        return Some(group.to_vec());
    }
    let gap = after_dash.len() - value.len();
    let content_indent = indent + 1 + gap;
    let mut virtual_lines = vec![format!("{}{value}", " ".repeat(content_indent))];
    virtual_lines.extend(nested.iter().cloned());
    let mut lines = reorder_node(&virtual_lines, permutation)?;
    let head = lines.first_mut()?;
    let content = head.get(content_indent..)?.to_owned();
    *head = format!("{}-{}{content}", " ".repeat(indent), " ".repeat(gap));
    Some(lines)
}

/// Values whose lines are kept: block scalars, anchors, aliases and tags.
fn is_left_unchanged(value: &str) -> bool {
    value.starts_with(['|', '>', '&', '*', '!'])
}

/// A value on one line: a flow collection, or a scalar that may embed JSON.
fn reorder_value(value: &str, permutation: &mut Permutation) -> String {
    if value.starts_with(['{', '[']) {
        reorder_flow(value, permutation).unwrap_or_else(|| value.to_owned())
    } else {
        reorder_embedded(value, permutation)
    }
}

/// A node that is neither a mapping nor a sequence: one line is reordered, more are kept.
fn reorder_value_lines(
    lines: &[String],
    indent: usize,
    permutation: &mut Permutation,
) -> Vec<String> {
    let significant = lines.iter().filter(|line| !is_blank(line)).count();
    match lines.split_first() {
        Some((first, rest)) if significant == 1 => {
            let value = first.get(indent..).unwrap_or_default();
            let mut result = vec![format!(
                "{}{}",
                " ".repeat(indent),
                reorder_value(value, permutation)
            )];
            result.extend(rest.iter().cloned());
            result
        }
        _ => lines.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::determinism::permutation::SEED;

    fn reorder(text: &str) -> Option<String> {
        reorder_block(text, &mut Permutation::new(SEED))
    }

    fn read(text: &str) -> serde_json::Value {
        serde_saphyr::from_str(text).unwrap()
    }

    #[test]
    fn pln_2_4_block_mappings_are_reordered_recursively() {
        let text = "a: 1\nb:\n  c: x\n  d: [1, 2]\n  e:\n  - f: 1\n    g: 2\n  - h\nz: |\n  text\n";
        let reordered = reorder(text).unwrap();
        assert_ne!(reordered, text);
        assert_eq!(read(&reordered), read(text));
        assert!(reordered.ends_with('\n'));
    }

    #[test]
    fn pln_2_4_sequence_items_keep_their_order() {
        let text = "- name: one\n  value: 1\n- name: two\n  value: 2\n";
        let reordered = reorder(text).unwrap();
        assert_eq!(read(&reordered), read(text));
        let one = reordered.find("one").unwrap();
        assert!(one < reordered.find("two").unwrap());
    }

    #[test]
    fn pln_2_4_documents_with_markers_are_kept() {
        assert_eq!(reorder("---\na: 1\n"), None);
        assert_eq!(reorder("a:\n\tb: 1\n"), None);
    }
}

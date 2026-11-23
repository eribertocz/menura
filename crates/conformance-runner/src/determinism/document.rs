//! Reorders the mappings of one input document, and keeps the reordering only when the
//! document still reads as the same data.

use serde_json::Value;

use super::block::reorder_block;
use super::flow::reorder_flow;
use super::permutation::Permutation;

/// `bytes`, a JSON or YAML document, with the entries of every mapping reordered, or `None`
/// when the document cannot be reordered safely: it is not UTF-8, does not read as YAML, uses
/// a form the reorderers keep, or would read differently once reordered (an alias moved
/// before its anchor, for example).
#[must_use]
pub fn reorder_document(bytes: &[u8], permutation: &mut Permutation) -> Option<Vec<u8>> {
    let text = std::str::from_utf8(bytes).ok()?;
    let original = read(text)?;
    let reordered = if text.trim_start().starts_with(['{', '[']) {
        let ending = if text.ends_with('\n') { "\n" } else { "" };
        reorder_flow(text, permutation).map(|reordered| format!("{reordered}{ending}"))?
    } else {
        reorder_block(text, permutation)?
    };
    (reordered != text && read(&reordered).as_ref() == Some(&original))
        .then(|| reordered.into_bytes())
}

/// The data of a document, with every string that holds a JSON object read as that object,
/// so that a reordered embedded document compares equal.
fn read(text: &str) -> Option<Value> {
    serde_saphyr::from_str::<Value>(text).ok().map(expand)
}

/// `value` with every string that holds a JSON object read as that object, recursively.
pub(crate) fn expand(value: Value) -> Value {
    match value {
        Value::String(text) if text.trim_start().starts_with('{') => {
            serde_json::from_str::<Value>(&text).map_or(Value::String(text), expand)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(expand).collect()),
        Value::Object(entries) => Value::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key, expand(value)))
                .collect(),
        ),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::determinism::permutation::SEED;

    #[test]
    fn pln_2_4_a_reordered_document_reads_as_the_same_data() {
        let text = "a: 1\nb: '{\"x\": 1, \"y\": 2}'\nc: {d: 1, e: 2}\n";
        let reordered = reorder_document(text.as_bytes(), &mut Permutation::new(SEED)).unwrap();
        let reordered = String::from_utf8(reordered).unwrap();
        assert_ne!(reordered, text);
        assert_eq!(read(&reordered), read(text));
    }

    #[test]
    fn pln_2_4_json_is_reordered_as_json() {
        let text = "{\n  \"a\": 1,\n  \"b\": 2\n}\n";
        let reordered = reorder_document(text.as_bytes(), &mut Permutation::new(SEED)).unwrap();
        assert_eq!(reordered, b"{\"b\": 2, \"a\": 1}\n");
    }

    #[test]
    fn pln_2_4_a_moved_alias_keeps_the_document() {
        let text = "a: &anchor 1\nb: *anchor\n";
        assert_eq!(
            reorder_document(text.as_bytes(), &mut Permutation::new(SEED)),
            None
        );
        assert_eq!(reorder_document(b"\xff", &mut Permutation::new(SEED)), None);
    }
}

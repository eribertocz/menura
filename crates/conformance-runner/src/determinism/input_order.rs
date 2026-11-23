//! The parts of an output document that the specification derives from the text of an input,
//! so that reordering the keys of that input legitimately changes them:
//!
//! - `line` and `column` of a `location`, the position of the text in its document;
//! - the order of `errors`, `warnings` and `changes`, which follow document order (E0-4.3,
//!   MIG-3.4);
//! - the model `migrate` returns, kept as read (MIG-3.3): its member order, and the text of
//!   the JSON objects it embeds in strings (extension `data`), which are reordered too.
//!
//! Two documents that differ only in these parts are equal up to input order.

use serde_json::Value;

use super::document::expand;

/// The lists ordered by the position of their element in an input document.
const DOCUMENT_ORDER_LISTS: [&str; 3] = ["errors", "warnings", "changes"];

/// The members of a `location` that give a position in the text of its document.
const TEXT_POSITIONS: [&str; 2] = ["line", "column"];

/// Whether two JSON documents are equal once the parts derived from input order are set
/// aside. Member order is set aside too, since a JSON value is compared as data; a text that
/// is not JSON is never equal up to input order to another text.
#[must_use]
pub fn equal_up_to_input_order(left: &str, right: &str) -> bool {
    let read = |text: &str| serde_json::from_str::<Value>(text).ok().map(normalize);
    matches!((read(left), read(right)), (Some(left), Some(right)) if left == right)
}

fn normalize(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(normalize).collect()),
        Value::Object(entries) => Value::Object(
            entries
                .into_iter()
                .map(|(key, value)| {
                    let value = normalize(value);
                    let value = match value {
                        model if key == "model" => expand(model),
                        Value::Object(mut location) if key == "location" => {
                            for position in TEXT_POSITIONS {
                                location.remove(position);
                            }
                            Value::Object(location)
                        }
                        Value::Array(mut items) if DOCUMENT_ORDER_LISTS.contains(&key.as_str()) => {
                            items.sort_by_cached_key(Value::to_string);
                            Value::Array(items)
                        }
                        other => other,
                    };
                    (key, value)
                })
                .collect(),
        ),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pln_2_4_text_positions_and_document_order_are_set_aside() {
        let left = r#"{"errors": [{"code": "A", "location": {"pointer": "/a", "line": 2}},
            {"code": "B", "location": null}], "model": {"x": 1, "y": 2}}"#;
        let right = r#"{"model": {"y": 2, "x": 1}, "errors": [{"code": "B", "location": null},
            {"code": "A", "location": {"line": 9, "pointer": "/a"}}]}"#;
        assert!(equal_up_to_input_order(left, right));
        let left = r#"{"model": {"data": "{\"a\": 1, \"b\": 2}"}}"#;
        let right = r#"{"model": {"data": "{\"b\": 2, \"a\": 1}"}}"#;
        assert!(equal_up_to_input_order(left, right));
    }

    #[test]
    fn pln_2_4_any_other_difference_remains() {
        let left = r#"{"cache_key": "a", "location": {"pointer": "/a"}}"#;
        assert!(!equal_up_to_input_order(
            left,
            r#"{"cache_key": "b", "location": {"pointer": "/a"}}"#
        ));
        assert!(!equal_up_to_input_order(
            left,
            r#"{"cache_key": "a", "location": {"pointer": "/b"}}"#
        ));
        assert!(!equal_up_to_input_order(
            r#"{"sql": [1, 2]}"#,
            r#"{"sql": [2, 1]}"#
        ));
        assert!(!equal_up_to_input_order(
            "usage error: a",
            "usage error: a "
        ));
    }
}

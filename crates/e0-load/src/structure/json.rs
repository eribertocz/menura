//! The JSON form of the located data tree, which the JSON Schema evaluator reads.

use serde_json::{Map, Number, Value as Json};

use crate::pointer::key_text;
use crate::read::{Integer, Node, Value};

/// The JSON form of `node`.
///
/// Keys that are not strings become their JSON text; when two keys have the same text, the first
/// one is kept, as [`crate::pointer`] resolves them. A number without an exact JSON form (an
/// integer beyond 64 bits, `.inf`, `.nan`) becomes a stand-in number: the official schema never
/// constrains numbers, so only their JSON type takes part in validation.
pub(crate) fn to_json(node: &Node) -> Json {
    match &node.value {
        Value::Null => Json::Null,
        Value::Boolean(value) => Json::Bool(*value),
        Value::Integer(value) => Json::Number(integer_number(value)),
        Value::Float(value) => Json::Number(Number::from_f64(*value).unwrap_or_else(stand_in)),
        Value::String(text) => Json::String(text.clone()),
        Value::Sequence(items) => Json::Array(items.iter().map(to_json).collect()),
        Value::Mapping(entries) => {
            let mut members = Map::new();
            for entry in entries.iter() {
                members
                    .entry(key_text(&entry.key))
                    .or_insert_with(|| to_json(&entry.value));
            }
            Json::Object(members)
        }
    }
}

fn integer_number(value: &Integer) -> Number {
    let magnitude: Option<u64> = value.magnitude().parse().ok();
    match magnitude {
        Some(magnitude) if !value.is_negative() => Number::from(magnitude),
        Some(magnitude) => {
            i64::try_from(-i128::from(magnitude)).map_or_else(|_| stand_in(), Number::from)
        }
        None => stand_in(),
    }
}

/// The number that stands in for one without an exact JSON form.
fn stand_in() -> Number {
    Number::from(0)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::to_json;
    use crate::read::read_document;

    #[test]
    fn the_json_form_keeps_values_and_writes_keys_as_text() {
        let document = read_document(b"a: [1, -2, 1.5, yes, ~, text]\n1: one\n~: n\n")
            .expect("the document is read");
        assert_eq!(
            to_json(&document),
            json!({"a": [1, -2, 1.5, true, null, "text"], "1": "one", "null": "n"})
        );
    }

    #[test]
    fn numbers_without_a_json_form_become_numbers() {
        let document = read_document(b"[.inf, .nan, 123456789012345678901234567890, -9223372036854775808, -9223372036854775809]\n")
            .expect("the document is read");
        assert_eq!(to_json(&document), json!([0, 0, 0, i64::MIN, 0]));
    }
}

//! Reading the object `data` of a development-line entry as the JSON value it represents
//! (E0C-2.8, MDL-2.4).

use serde_json::{Map, Number, Value as Json};

use crate::pointer::key_text;
use crate::read::{Integer, Node, Value};

/// The JSON value that `node` represents.
///
/// A YAML `null` is the JSON `null`. An integer is read as the JSON reader of the release
/// format reads the same digits, so both formats give the same value (E0C-2.8).
///
/// # Errors
///
/// Returns why `node` has no JSON form: a mapping key that is not a string, or a number with no
/// JSON form (`.inf`, `.nan`). The caller reports it as `E0C-03`.
pub(crate) fn json_value(node: &Node) -> Result<Json, String> {
    match &node.value {
        Value::Null => Ok(Json::Null),
        Value::Boolean(value) => Ok(Json::Bool(*value)),
        Value::Integer(value) => integer(value),
        Value::Float(value) => Number::from_f64(*value)
            .map(Json::Number)
            .ok_or_else(|| format!("the number {value} has no JSON form")),
        Value::String(text) => Ok(Json::String(text.clone())),
        Value::Sequence(items) => items
            .iter()
            .map(json_value)
            .collect::<Result<Vec<_>, _>>()
            .map(Json::Array),
        Value::Mapping(entries) => {
            let mut members = Map::new();
            for entry in entries.iter() {
                let Value::String(key) = &entry.key.value else {
                    return Err(format!(
                        "the key `{}` is not a string",
                        key_text(&entry.key)
                    ));
                };
                members.insert(key.clone(), json_value(&entry.value)?);
            }
            Ok(Json::Object(members))
        }
    }
}

/// The JSON value the release format's JSON reader gives for the digits of `value`.
fn integer(value: &Integer) -> Result<Json, String> {
    let digits = value.to_string();
    serde_json::from_str(&digits)
        .map_err(|error| format!("the integer {digits} has no JSON form ({error})"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::json_value;
    use crate::read::read_document;

    fn read(text: &str) -> Result<serde_json::Value, String> {
        let document = read_document(text.as_bytes()).expect("the document is read");
        json_value(&document)
    }

    #[test]
    fn e0c_2_8_a_yaml_object_is_read_as_the_json_value_it_represents() {
        assert_eq!(
            read("{a: [1, -2, 1.5, yes, ~, text, '1'], b: {c: null}}\n"),
            Ok(json!({"a": [1, -2, 1.5, true, null, "text", "1"], "b": {"c": null}}))
        );
    }

    #[test]
    fn e0c_2_8_integers_are_read_as_the_json_reader_reads_their_digits() {
        let digits = "123456789012345678901234567890";
        let release: serde_json::Value = serde_json::from_str(digits).expect("JSON");
        assert_eq!(read(&format!("{digits}\n")), Ok(release));
        assert_eq!(read("0x10\n"), Ok(json!(16)));
        assert_eq!(read("-9223372036854775808\n"), Ok(json!(i64::MIN)));
    }

    #[test]
    fn e0c_2_8_keys_that_are_not_strings_and_numbers_without_json_form_have_no_json_value() {
        for text in [
            "{1: a}\n",
            "{~: a}\n",
            "{a: {true: b}}\n",
            "[.inf]\n",
            "{a: -.inf}\n",
            "{a: [.nan]}\n",
        ] {
            assert!(read(text).is_err(), "{text}");
        }
    }
}

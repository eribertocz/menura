//! The indented JSON text of an [`OrderedJson`] value, written by hand so that integers of any
//! size keep their exact digits (MIG-3.3a): a serializer writes numbers of at most 128 bits.
//! Every scalar but an integer is written by `serde_json`, so the text has the same form as
//! the other output documents: two-space indentation, `": "` between a name and its value.

use std::fmt::Write;

use menura_contracts::types::ordered_json::OrderedJson;

/// The JSON text of `value`, indented with two spaces.
///
/// # Errors
///
/// The error of `serde_json` when a scalar cannot be written.
pub(crate) fn pretty_text(value: &OrderedJson) -> Result<String, serde_json::Error> {
    let mut text = String::new();
    write_value(&mut text, value, 0)?;
    Ok(text)
}

fn write_value(
    text: &mut String,
    value: &OrderedJson,
    depth: usize,
) -> Result<(), serde_json::Error> {
    match value {
        OrderedJson::Integer(integer) => text.push_str(integer.as_str()),
        OrderedJson::Array(items) if items.is_empty() => text.push_str("[]"),
        OrderedJson::Array(items) => {
            text.push('[');
            for (index, item) in items.iter().enumerate() {
                separate(text, index, depth + 1);
                write_value(text, item, depth + 1)?;
            }
            close(text, ']', depth);
        }
        OrderedJson::Object(object) if object.members().is_empty() => text.push_str("{}"),
        OrderedJson::Object(object) => {
            text.push('{');
            for (index, (name, member)) in object.members().iter().enumerate() {
                separate(text, index, depth + 1);
                text.push_str(&serde_json::to_string(name)?);
                text.push_str(": ");
                write_value(text, member, depth + 1)?;
            }
            close(text, '}', depth);
        }
        scalar => text.push_str(&serde_json::to_string(scalar)?),
    }
    Ok(())
}

/// Starts the item `index` of a container at `depth`.
fn separate(text: &mut String, index: usize, depth: usize) {
    if index > 0 {
        text.push(',');
    }
    newline(text, depth);
}

/// Closes a container at `depth` with `bracket`.
fn close(text: &mut String, bracket: char, depth: usize) {
    newline(text, depth);
    text.push(bracket);
}

fn newline(text: &mut String, depth: usize) {
    text.push('\n');
    let _ = write!(text, "{:width$}", "", width = depth * 2);
}

#[cfg(test)]
mod tests {
    use menura_contracts::types::ordered_json::{JsonInteger, OrderedObject};

    use super::*;

    #[test]
    fn the_text_has_the_form_of_serde_json_pretty_output() {
        let text = r#"{"b": [1, {"c": "\u0001\"x"}, [], {}], "a": -1.5e300, "n": null, "t": true}"#;
        let value: OrderedJson = serde_json::from_str(text).unwrap();
        let expected = serde_json::to_string_pretty(&value).unwrap();
        assert_eq!(pretty_text(&value).unwrap(), expected);
    }

    #[test]
    fn mig_3_3a_integers_of_any_size_keep_their_digits() {
        let digits = "-123456789012345678901234567890123456789012";
        let value = OrderedJson::Object(OrderedObject::new(vec![(
            "n".to_owned(),
            OrderedJson::Integer(JsonInteger::parse(digits).unwrap()),
        )]));
        assert_eq!(
            pretty_text(&value).unwrap(),
            format!("{{\n  \"n\": {digits}\n}}")
        );
    }
}

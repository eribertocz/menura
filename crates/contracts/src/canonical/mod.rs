//! RFC 8785 JSON Canonicalization Scheme (CON-2).

mod number;

use std::fmt::Write as _;

use serde_json::{Map, Number, Value};

/// Serializes `value` as RFC 8785 canonical JSON.
///
/// Object members are sorted by the UTF-16 code units of their names, strings use the minimal
/// escaping of RFC 8785 §3.2.2.2, and numbers are written as the IEEE-754 double closest to them
/// in the ECMAScript form (§3.2.2.3). Because of that, integers beyond ±(2^53 − 1) lose precision
/// as numbers; CON-4 requires such values to be strings.
///
/// # Example
///
/// ```
/// use menura_contracts::canonical_json;
///
/// let value = serde_json::json!({"b": [1.50, true], "a": "€"});
/// assert_eq!(canonical_json(&value), r#"{"a":"€","b":[1.5,true]}"#);
/// ```
#[must_use]
pub fn canonical_json(value: &Value) -> String {
    let mut output = String::new();
    write_value(value, &mut output);
    output
}

fn write_value(value: &Value, output: &mut String) {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(true) => output.push_str("true"),
        Value::Bool(false) => output.push_str("false"),
        Value::Number(number) => write_json_number(number, output),
        Value::String(text) => write_string(text, output),
        Value::Array(items) => write_array(items, output),
        Value::Object(members) => write_object(members, output),
    }
}

fn write_json_number(number: &Number, output: &mut String) {
    number::write_number(number.as_f64().unwrap_or(f64::NAN), output);
}

fn write_array(items: &[Value], output: &mut String) {
    output.push('[');
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        write_value(item, output);
    }
    output.push(']');
}

fn write_object(members: &Map<String, Value>, output: &mut String) {
    let mut sorted: Vec<(&String, &Value)> = members.iter().collect();
    sorted.sort_by(|left, right| left.0.encode_utf16().cmp(right.0.encode_utf16()));
    output.push('{');
    for (index, (name, member)) in sorted.into_iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        write_string(name, output);
        output.push(':');
        write_value(member, output);
    }
    output.push('}');
}

fn write_string(text: &str, output: &mut String) {
    output.push('"');
    for character in text.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{8}' => output.push_str("\\b"),
            '\u{9}' => output.push_str("\\t"),
            '\u{a}' => output.push_str("\\n"),
            '\u{c}' => output.push_str("\\f"),
            '\u{d}' => output.push_str("\\r"),
            control if u32::from(control) < 0x20 => {
                // Writing to a String cannot fail.
                let _ = write!(output, "\\u{:04x}", u32::from(control));
            }
            other => output.push(other),
        }
    }
    output.push('"');
}

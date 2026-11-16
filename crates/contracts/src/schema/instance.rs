//! JSON Schema views of an instance: its type names, equality and number comparison
//! (JSON Schema 2020-12 Core §4.2).

use std::cmp::Ordering;

use serde_json::{Number, Value};

/// Whether `instance` belongs to the JSON Schema type `type_name`.
/// A number with a zero fractional part is an `integer` (Core §4.2.1).
pub(crate) fn has_type(instance: &Value, type_name: &str) -> bool {
    match type_name {
        "null" => instance.is_null(),
        "boolean" => instance.is_boolean(),
        "object" => instance.is_object(),
        "array" => instance.is_array(),
        "string" => instance.is_string(),
        "number" => instance.is_number(),
        "integer" => instance.as_number().is_some_and(is_integral),
        _ => false,
    }
}

/// Whether `number` has no fractional part.
pub(crate) fn is_integral(number: &Number) -> bool {
    number.is_i64() || number.is_u64() || number.as_f64().is_some_and(|value| value.fract() == 0.0)
}

/// JSON equality: numbers compare by mathematical value, objects ignore member order.
pub(crate) fn json_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => {
            compare_numbers(left, right) == Some(Ordering::Equal)
        }
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| json_equal(left, right))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left.iter().all(|(name, member)| {
                    right
                        .get(name)
                        .is_some_and(|other| json_equal(member, other))
                })
        }
        _ => left == right,
    }
}

/// Compares two numbers exactly when both are integers, otherwise as doubles.
pub(crate) fn compare_numbers(left: &Number, right: &Number) -> Option<Ordering> {
    match (as_integer(left), as_integer(right)) {
        (Some(left), Some(right)) => Some(left.cmp(&right)),
        _ => left.as_f64()?.partial_cmp(&right.as_f64()?),
    }
}

fn as_integer(number: &Number) -> Option<i128> {
    number
        .as_i64()
        .map(i128::from)
        .or_else(|| number.as_u64().map(i128::from))
}

/// Appends one reference token to a JSON Pointer (RFC 6901 §3: `~` → `~0`, `/` → `~1`).
pub(crate) fn child_pointer(pointer: &str, token: &str) -> String {
    let escaped = token.replace('~', "~0").replace('/', "~1");
    format!("{pointer}/{escaped}")
}

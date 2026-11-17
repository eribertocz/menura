//! The keywords that look at one instance alone, without subschemas: type, value set, number
//! bounds, text and unique items.

use serde_json::{Map, Value};

use super::Found;
use super::violation;
use crate::schema::SchemaViolationKind;
use crate::schema::instance::{child_pointer, compare_numbers, has_type, json_equal};
use crate::schema::strings::{matches_format, matches_pattern};

pub(super) fn collect_type(
    keywords: &Map<String, Value>,
    instance: &Value,
    pointer: &str,
    found: &mut Found,
) {
    let accepted = match keywords.get("type") {
        None => return,
        Some(Value::String(name)) => has_type(instance, name),
        Some(Value::Array(names)) => names
            .iter()
            .filter_map(Value::as_str)
            .any(|name| has_type(instance, name)),
        Some(_) => false,
    };
    if !accepted {
        found.push(violation(
            pointer,
            SchemaViolationKind::Type,
            "the value has the wrong type",
        ));
    }
}

pub(super) fn collect_value_set(
    keywords: &Map<String, Value>,
    instance: &Value,
    pointer: &str,
    found: &mut Found,
) {
    if let Some(constant) = keywords.get("const")
        && !json_equal(constant, instance)
    {
        found.push(violation(
            pointer,
            SchemaViolationKind::Const,
            &format!("the value must be {constant}"),
        ));
    }
    if let Some(Value::Array(allowed)) = keywords.get("enum")
        && !allowed
            .iter()
            .any(|candidate| json_equal(candidate, instance))
    {
        found.push(violation(
            pointer,
            SchemaViolationKind::Enum,
            "the value is not one of the allowed values",
        ));
    }
}

pub(super) fn collect_bounds(
    keywords: &Map<String, Value>,
    instance: &Value,
    pointer: &str,
    found: &mut Found,
) {
    let Value::Number(number) = instance else {
        return;
    };
    if let Some(Value::Number(minimum)) = keywords.get("minimum")
        && compare_numbers(number, minimum).is_none_or(std::cmp::Ordering::is_lt)
    {
        found.push(violation(
            pointer,
            SchemaViolationKind::Minimum,
            &format!("the value must be at least {minimum}"),
        ));
    }
    if let Some(Value::Number(maximum)) = keywords.get("maximum")
        && compare_numbers(number, maximum).is_none_or(std::cmp::Ordering::is_gt)
    {
        found.push(violation(
            pointer,
            SchemaViolationKind::Maximum,
            &format!("the value must be at most {maximum}"),
        ));
    }
}

pub(super) fn collect_text(
    keywords: &Map<String, Value>,
    instance: &Value,
    pointer: &str,
    found: &mut Found,
) {
    let Value::String(text) = instance else {
        return;
    };
    let length = text.chars().count() as u64;
    if keywords
        .get("minLength")
        .and_then(Value::as_u64)
        .is_some_and(|minimum| length < minimum)
    {
        found.push(violation(
            pointer,
            SchemaViolationKind::MinLength,
            "the text is too short",
        ));
    }
    if keywords
        .get("maxLength")
        .and_then(Value::as_u64)
        .is_some_and(|maximum| length > maximum)
    {
        found.push(violation(
            pointer,
            SchemaViolationKind::MaxLength,
            "the text is too long",
        ));
    }
    if let Some(Value::String(pattern)) = keywords.get("pattern")
        && matches_pattern(pattern, text) != Some(true)
    {
        found.push(violation(
            pointer,
            SchemaViolationKind::Pattern,
            "the text does not have the required form",
        ));
    }
    if let Some(Value::String(format)) = keywords.get("format")
        && matches_format(format, text) != Some(true)
    {
        found.push(violation(
            pointer,
            SchemaViolationKind::Format,
            &format!("the text is not a valid {format}"),
        ));
    }
}

pub(super) fn collect_unique(items: &[Value], pointer: &str, found: &mut Found) {
    for (index, item) in items.iter().enumerate() {
        if items[..index]
            .iter()
            .any(|earlier| json_equal(earlier, item))
        {
            found.push(violation(
                &child_pointer(pointer, &index.to_string()),
                SchemaViolationKind::UniqueItems,
                "the array items must be unique",
            ));
            return;
        }
    }
}

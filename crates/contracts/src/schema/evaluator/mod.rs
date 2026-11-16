//! Evaluates an instance against a JSON Schema 2020-12 schema, for the keywords the contract
//! schemas use. Any other assertion keyword fails closed.

use serde_json::{Map, Value};

use super::SchemaViolation;
use super::instance::{child_pointer, compare_numbers, has_type, json_equal};
use super::strings::{matches_format, matches_pattern};

/// Keywords that carry no assertion and are ignored during evaluation.
pub(crate) const ANNOTATION_KEYWORDS: [&str; 6] =
    ["$schema", "$id", "$defs", "title", "description", "default"];

/// Assertion and applicator keywords this evaluator implements.
pub(crate) const ASSERTION_KEYWORDS: [&str; 24] = [
    "$ref",
    "type",
    "const",
    "enum",
    "minimum",
    "maximum",
    "minLength",
    "maxLength",
    "pattern",
    "format",
    "minItems",
    "maxItems",
    "uniqueItems",
    "items",
    "required",
    "properties",
    "additionalProperties",
    "allOf",
    "anyOf",
    "oneOf",
    "not",
    "if",
    "then",
    "else",
];

/// Validates instances against subschemas of one root schema.
pub(crate) struct Evaluator<'schema> {
    root: &'schema Value,
}

type Outcome = Result<(), SchemaViolation>;

impl<'schema> Evaluator<'schema> {
    pub(crate) fn new(root: &'schema Value) -> Self {
        Self { root }
    }

    /// Validates `instance`, located at `pointer`, against `schema`.
    /// Returns the first violation, in keyword order then document order.
    pub(crate) fn validate(&self, schema: &Value, instance: &Value, pointer: &str) -> Outcome {
        let keywords = match schema {
            Value::Bool(true) => return Ok(()),
            Value::Object(keywords) => keywords,
            _ => {
                return Err(violation(
                    pointer,
                    "the instance is rejected by a false schema",
                ));
            }
        };
        for name in keywords.keys() {
            let known = ASSERTION_KEYWORDS.contains(&name.as_str())
                || ANNOTATION_KEYWORDS.contains(&name.as_str());
            if !known {
                return Err(violation(
                    pointer,
                    &format!("unsupported schema keyword `{name}`"),
                ));
            }
        }
        self.validate_reference(keywords, instance, pointer)?;
        validate_type(keywords, instance, pointer)?;
        validate_value_set(keywords, instance, pointer)?;
        validate_bounds(keywords, instance, pointer)?;
        validate_text(keywords, instance, pointer)?;
        self.validate_array(keywords, instance, pointer)?;
        self.validate_object(keywords, instance, pointer)?;
        self.validate_combinations(keywords, instance, pointer)?;
        self.validate_conditional(keywords, instance, pointer)
    }

    fn matches(&self, schema: &Value, instance: &Value, pointer: &str) -> bool {
        self.validate(schema, instance, pointer).is_ok()
    }

    fn validate_reference(
        &self,
        keywords: &Map<String, Value>,
        instance: &Value,
        pointer: &str,
    ) -> Outcome {
        let Some(reference) = keywords.get("$ref") else {
            return Ok(());
        };
        let target = reference
            .as_str()
            .and_then(|text| text.strip_prefix("#/$defs/"))
            .and_then(|name| self.root.get("$defs")?.get(name));
        match target {
            Some(target) => self.validate(target, instance, pointer),
            None => Err(violation(pointer, "the schema has an unresolvable `$ref`")),
        }
    }

    fn validate_array(
        &self,
        keywords: &Map<String, Value>,
        instance: &Value,
        pointer: &str,
    ) -> Outcome {
        let Value::Array(items) = instance else {
            return Ok(());
        };
        if let Some(minimum) = keywords.get("minItems").and_then(Value::as_u64)
            && (items.len() as u64) < minimum
        {
            return Err(violation(
                pointer,
                &format!("the array needs at least {minimum} items"),
            ));
        }
        if let Some(maximum) = keywords.get("maxItems").and_then(Value::as_u64)
            && (items.len() as u64) > maximum
        {
            return Err(violation(
                pointer,
                &format!("the array allows at most {maximum} items"),
            ));
        }
        if keywords.get("uniqueItems") == Some(&Value::Bool(true)) {
            validate_unique(items, pointer)?;
        }
        if let Some(item_schema) = keywords.get("items") {
            for (index, item) in items.iter().enumerate() {
                self.validate(
                    item_schema,
                    item,
                    &child_pointer(pointer, &index.to_string()),
                )?;
            }
        }
        Ok(())
    }

    fn validate_object(
        &self,
        keywords: &Map<String, Value>,
        instance: &Value,
        pointer: &str,
    ) -> Outcome {
        let Value::Object(members) = instance else {
            return Ok(());
        };
        if let Some(Value::Array(required)) = keywords.get("required") {
            for name in required.iter().filter_map(Value::as_str) {
                if !members.contains_key(name) {
                    return Err(violation(
                        pointer,
                        &format!("the required member `{name}` is missing"),
                    ));
                }
            }
        }
        let properties = keywords.get("properties").and_then(Value::as_object);
        for (name, member) in members {
            let member_pointer = child_pointer(pointer, name);
            match properties.and_then(|properties| properties.get(name)) {
                Some(member_schema) => self.validate(member_schema, member, &member_pointer)?,
                None => self.validate_additional(keywords, name, member, &member_pointer)?,
            }
        }
        Ok(())
    }

    fn validate_additional(
        &self,
        keywords: &Map<String, Value>,
        name: &str,
        member: &Value,
        pointer: &str,
    ) -> Outcome {
        match keywords.get("additionalProperties") {
            None => Ok(()),
            Some(Value::Bool(false)) => Err(violation(
                pointer,
                &format!("the member `{name}` is not allowed"),
            )),
            Some(schema) => self.validate(schema, member, pointer),
        }
    }

    fn validate_combinations(
        &self,
        keywords: &Map<String, Value>,
        instance: &Value,
        pointer: &str,
    ) -> Outcome {
        if let Some(Value::Array(all)) = keywords.get("allOf") {
            for schema in all {
                self.validate(schema, instance, pointer)?;
            }
        }
        if let Some(Value::Array(any)) = keywords.get("anyOf")
            && !any
                .iter()
                .any(|schema| self.matches(schema, instance, pointer))
        {
            return Err(violation(
                pointer,
                "the value matches none of the allowed alternatives",
            ));
        }
        if let Some(Value::Array(one)) = keywords.get("oneOf") {
            let matching = one
                .iter()
                .filter(|schema| self.matches(schema, instance, pointer))
                .count();
            if matching != 1 {
                return Err(violation(
                    pointer,
                    "the value must match exactly one of the allowed alternatives",
                ));
            }
        }
        if let Some(schema) = keywords.get("not")
            && self.matches(schema, instance, pointer)
        {
            return Err(violation(
                pointer,
                "the value matches a forbidden alternative",
            ));
        }
        Ok(())
    }

    fn validate_conditional(
        &self,
        keywords: &Map<String, Value>,
        instance: &Value,
        pointer: &str,
    ) -> Outcome {
        let Some(condition) = keywords.get("if") else {
            return Ok(());
        };
        let branch = if self.matches(condition, instance, pointer) {
            keywords.get("then")
        } else {
            keywords.get("else")
        };
        branch.map_or(Ok(()), |schema| self.validate(schema, instance, pointer))
    }
}

fn validate_type(keywords: &Map<String, Value>, instance: &Value, pointer: &str) -> Outcome {
    let accepted = match keywords.get("type") {
        None => return Ok(()),
        Some(Value::String(name)) => has_type(instance, name),
        Some(Value::Array(names)) => names
            .iter()
            .filter_map(Value::as_str)
            .any(|name| has_type(instance, name)),
        Some(_) => false,
    };
    if accepted {
        Ok(())
    } else {
        Err(violation(pointer, "the value has the wrong type"))
    }
}

fn validate_value_set(keywords: &Map<String, Value>, instance: &Value, pointer: &str) -> Outcome {
    if let Some(constant) = keywords.get("const")
        && !json_equal(constant, instance)
    {
        return Err(violation(pointer, &format!("the value must be {constant}")));
    }
    if let Some(Value::Array(allowed)) = keywords.get("enum")
        && !allowed
            .iter()
            .any(|candidate| json_equal(candidate, instance))
    {
        return Err(violation(
            pointer,
            "the value is not one of the allowed values",
        ));
    }
    Ok(())
}

fn validate_bounds(keywords: &Map<String, Value>, instance: &Value, pointer: &str) -> Outcome {
    let Value::Number(number) = instance else {
        return Ok(());
    };
    if let Some(Value::Number(minimum)) = keywords.get("minimum")
        && compare_numbers(number, minimum).is_none_or(std::cmp::Ordering::is_lt)
    {
        return Err(violation(
            pointer,
            &format!("the value must be at least {minimum}"),
        ));
    }
    if let Some(Value::Number(maximum)) = keywords.get("maximum")
        && compare_numbers(number, maximum).is_none_or(std::cmp::Ordering::is_gt)
    {
        return Err(violation(
            pointer,
            &format!("the value must be at most {maximum}"),
        ));
    }
    Ok(())
}

fn validate_text(keywords: &Map<String, Value>, instance: &Value, pointer: &str) -> Outcome {
    let Value::String(text) = instance else {
        return Ok(());
    };
    let length = text.chars().count() as u64;
    if keywords
        .get("minLength")
        .and_then(Value::as_u64)
        .is_some_and(|minimum| length < minimum)
    {
        return Err(violation(pointer, "the text is too short"));
    }
    if keywords
        .get("maxLength")
        .and_then(Value::as_u64)
        .is_some_and(|maximum| length > maximum)
    {
        return Err(violation(pointer, "the text is too long"));
    }
    if let Some(Value::String(pattern)) = keywords.get("pattern")
        && matches_pattern(pattern, text) != Some(true)
    {
        return Err(violation(
            pointer,
            "the text does not have the required form",
        ));
    }
    if let Some(Value::String(format)) = keywords.get("format")
        && matches_format(format, text) != Some(true)
    {
        return Err(violation(
            pointer,
            &format!("the text is not a valid {format}"),
        ));
    }
    Ok(())
}

fn validate_unique(items: &[Value], pointer: &str) -> Outcome {
    for (index, item) in items.iter().enumerate() {
        if items[..index]
            .iter()
            .any(|earlier| json_equal(earlier, item))
        {
            return Err(violation(
                &child_pointer(pointer, &index.to_string()),
                "the array items must be unique",
            ));
        }
    }
    Ok(())
}

fn violation(pointer: &str, message: &str) -> SchemaViolation {
    SchemaViolation {
        pointer: pointer.to_owned(),
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests;

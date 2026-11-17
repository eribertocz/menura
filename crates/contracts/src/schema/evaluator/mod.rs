//! Evaluates an instance against a JSON Schema 2020-12 schema, for the keywords the contract
//! schemas use. Any other assertion keyword fails closed.

use serde_json::{Map, Value};

mod local;

use super::instance::child_pointer;
use super::{SchemaViolation, SchemaViolationKind};
use local::{collect_bounds, collect_text, collect_type, collect_unique, collect_value_set};

/// Keywords that carry no assertion and are ignored during evaluation: the identifiers and
/// definitions, and the annotation keywords of JSON Schema 2020-12.
pub(crate) const ANNOTATION_KEYWORDS: [&str; 11] = [
    "$schema",
    "$id",
    "$defs",
    "$comment",
    "title",
    "description",
    "default",
    "deprecated",
    "readOnly",
    "writeOnly",
    "examples",
];

/// The only `$ref` form resolved: a JSON Pointer (RFC 6901) into `$defs` of the root schema.
const DEFINITIONS_REFERENCE_PREFIX: &str = "#/$defs/";

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

/// Where violations are collected, in evaluation order.
pub(crate) type Found = Vec<SchemaViolation>;

impl<'schema> Evaluator<'schema> {
    pub(crate) fn new(root: &'schema Value) -> Self {
        Self { root }
    }

    /// Validates `instance`, located at `pointer`, against `schema`.
    /// Returns the first violation, in keyword order then document order.
    pub(crate) fn validate(
        &self,
        schema: &Value,
        instance: &Value,
        pointer: &str,
    ) -> Result<(), SchemaViolation> {
        self.violations(schema, instance, pointer)
            .into_iter()
            .next()
            .map_or(Ok(()), Err)
    }

    /// Every violation of `instance`, located at `pointer`, against `schema`, in keyword order
    /// then document order. The first one is the one [`Self::validate`] returns.
    pub(crate) fn violations(&self, schema: &Value, instance: &Value, pointer: &str) -> Found {
        let mut found = Vec::new();
        self.collect(schema, instance, pointer, &mut found);
        found
    }

    fn collect(&self, schema: &Value, instance: &Value, pointer: &str, found: &mut Found) {
        let keywords = match schema {
            Value::Bool(true) => return,
            Value::Object(keywords) => keywords,
            _ => {
                found.push(violation(
                    pointer,
                    SchemaViolationKind::FalseSchema,
                    "the instance is rejected by a false schema",
                ));
                return;
            }
        };
        for name in keywords.keys() {
            let known = ASSERTION_KEYWORDS.contains(&name.as_str())
                || ANNOTATION_KEYWORDS.contains(&name.as_str());
            if !known {
                found.push(violation(
                    pointer,
                    SchemaViolationKind::UnsupportedSchema,
                    &format!("unsupported schema keyword `{name}`"),
                ));
                return;
            }
        }
        self.collect_reference(keywords, instance, pointer, found);
        collect_type(keywords, instance, pointer, found);
        collect_value_set(keywords, instance, pointer, found);
        collect_bounds(keywords, instance, pointer, found);
        collect_text(keywords, instance, pointer, found);
        self.collect_array(keywords, instance, pointer, found);
        self.collect_object(keywords, instance, pointer, found);
        self.collect_combinations(keywords, instance, pointer, found);
        self.collect_conditional(keywords, instance, pointer, found);
    }

    fn matches(&self, schema: &Value, instance: &Value, pointer: &str) -> bool {
        self.violations(schema, instance, pointer).is_empty()
    }

    fn collect_reference(
        &self,
        keywords: &Map<String, Value>,
        instance: &Value,
        pointer: &str,
        found: &mut Found,
    ) {
        let Some(reference) = keywords.get("$ref") else {
            return;
        };
        match reference.as_str().and_then(|text| self.resolve(text)) {
            Some(target) => self.collect(target, instance, pointer, found),
            None => found.push(violation(
                pointer,
                SchemaViolationKind::UnsupportedSchema,
                "the schema has an unresolvable `$ref`",
            )),
        }
    }

    /// The subschema a `$ref` names: a JSON Pointer into `$defs` of the root schema, such as
    /// `#/$defs/Model` or `#/$defs/Model/properties/name`. Any other form, and a fragment with
    /// percent-encoding, is not resolved.
    fn resolve(&self, reference: &str) -> Option<&'schema Value> {
        if !reference.starts_with(DEFINITIONS_REFERENCE_PREFIX) || reference.contains('%') {
            return None;
        }
        self.root.pointer(reference.strip_prefix('#')?)
    }

    fn collect_array(
        &self,
        keywords: &Map<String, Value>,
        instance: &Value,
        pointer: &str,
        found: &mut Found,
    ) {
        let Value::Array(items) = instance else {
            return;
        };
        if let Some(minimum) = keywords.get("minItems").and_then(Value::as_u64)
            && (items.len() as u64) < minimum
        {
            found.push(violation(
                pointer,
                SchemaViolationKind::MinItems,
                &format!("the array needs at least {minimum} items"),
            ));
        }
        if let Some(maximum) = keywords.get("maxItems").and_then(Value::as_u64)
            && (items.len() as u64) > maximum
        {
            found.push(violation(
                pointer,
                SchemaViolationKind::MaxItems,
                &format!("the array allows at most {maximum} items"),
            ));
        }
        if keywords.get("uniqueItems") == Some(&Value::Bool(true)) {
            collect_unique(items, pointer, found);
        }
        if let Some(item_schema) = keywords.get("items") {
            for (index, item) in items.iter().enumerate() {
                let item_pointer = child_pointer(pointer, &index.to_string());
                self.collect(item_schema, item, &item_pointer, found);
            }
        }
    }

    fn collect_object(
        &self,
        keywords: &Map<String, Value>,
        instance: &Value,
        pointer: &str,
        found: &mut Found,
    ) {
        let Value::Object(members) = instance else {
            return;
        };
        if let Some(Value::Array(required)) = keywords.get("required") {
            for name in required.iter().filter_map(Value::as_str) {
                if !members.contains_key(name) {
                    found.push(violation(
                        pointer,
                        SchemaViolationKind::Required(name.to_owned()),
                        &format!("the required member `{name}` is missing"),
                    ));
                }
            }
        }
        let properties = keywords.get("properties").and_then(Value::as_object);
        for (name, member) in members {
            let member_pointer = child_pointer(pointer, name);
            match properties.and_then(|properties| properties.get(name)) {
                Some(member_schema) => self.collect(member_schema, member, &member_pointer, found),
                None => self.collect_additional(keywords, name, member, &member_pointer, found),
            }
        }
    }

    fn collect_additional(
        &self,
        keywords: &Map<String, Value>,
        name: &str,
        member: &Value,
        pointer: &str,
        found: &mut Found,
    ) {
        match keywords.get("additionalProperties") {
            None => {}
            Some(Value::Bool(false)) => found.push(violation(
                pointer,
                SchemaViolationKind::AdditionalProperties,
                &format!("the member `{name}` is not allowed"),
            )),
            Some(schema) => self.collect(schema, member, pointer, found),
        }
    }

    fn collect_combinations(
        &self,
        keywords: &Map<String, Value>,
        instance: &Value,
        pointer: &str,
        found: &mut Found,
    ) {
        if let Some(Value::Array(all)) = keywords.get("allOf") {
            for schema in all {
                self.collect(schema, instance, pointer, found);
            }
        }
        if let Some(Value::Array(any)) = keywords.get("anyOf")
            && !any
                .iter()
                .any(|schema| self.matches(schema, instance, pointer))
        {
            found.push(violation(
                pointer,
                SchemaViolationKind::AnyOf,
                "the value matches none of the allowed alternatives",
            ));
        }
        if let Some(Value::Array(one)) = keywords.get("oneOf") {
            let matching = one
                .iter()
                .filter(|schema| self.matches(schema, instance, pointer))
                .count();
            if matching != 1 {
                found.push(violation(
                    pointer,
                    SchemaViolationKind::OneOf,
                    "the value must match exactly one of the allowed alternatives",
                ));
            }
        }
        if let Some(schema) = keywords.get("not")
            && self.matches(schema, instance, pointer)
        {
            found.push(violation(
                pointer,
                SchemaViolationKind::Not,
                "the value matches a forbidden alternative",
            ));
        }
    }

    fn collect_conditional(
        &self,
        keywords: &Map<String, Value>,
        instance: &Value,
        pointer: &str,
        found: &mut Found,
    ) {
        let Some(condition) = keywords.get("if") else {
            return;
        };
        let branch = if self.matches(condition, instance, pointer) {
            keywords.get("then")
        } else {
            keywords.get("else")
        };
        if let Some(schema) = branch {
            self.collect(schema, instance, pointer, found);
        }
    }
}

pub(crate) fn violation(
    pointer: &str,
    kind: SchemaViolationKind,
    message: &str,
) -> SchemaViolation {
    SchemaViolation {
        pointer: pointer.to_owned(),
        kind,
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests;

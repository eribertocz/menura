//! Tests of the evaluator, on synthetic schemas and on the published ones.

use serde_json::{Value, json};

use super::{ANNOTATION_KEYWORDS, ASSERTION_KEYWORDS, Evaluator};
use crate::contract::ContractKind;
use crate::schema::SchemaViolationKind;
use crate::schema::schema_text;

/// Collects the keywords of `schema` and of every subschema below it.
fn keywords(schema: &Value, found: &mut Vec<String>) {
    let Value::Object(members) = schema else {
        return;
    };
    for (name, member) in members {
        found.push(name.clone());
        match name.as_str() {
            "properties" | "$defs" => {
                for subschema in member.as_object().into_iter().flat_map(|map| map.values()) {
                    keywords(subschema, found);
                }
            }
            "allOf" | "anyOf" | "oneOf" => {
                for subschema in member.as_array().into_iter().flatten() {
                    keywords(subschema, found);
                }
            }
            "items" | "additionalProperties" | "not" | "if" | "then" | "else" => {
                keywords(member, found);
            }
            _ => {}
        }
    }
}

fn pointer_of_violation(schema: &Value, instance: &Value) -> Option<String> {
    Evaluator::new(schema)
        .validate(schema, instance, "")
        .err()
        .map(|violation| violation.pointer)
}

#[test]
fn one_of_requires_exactly_one_match() {
    let schema = json!({"oneOf": [{"type": "integer"}, {"type": "number"}]});
    assert_eq!(
        pointer_of_violation(&schema, &json!(1)),
        Some(String::new())
    );
    assert_eq!(pointer_of_violation(&schema, &json!(1.5)), None);
    assert_eq!(
        pointer_of_violation(&schema, &json!("x")),
        Some(String::new())
    );
}

#[test]
fn integral_numbers_are_integers_and_compare_by_value() {
    let schema = json!({"type": "integer", "enum": [8, 16], "maximum": 16});
    assert_eq!(pointer_of_violation(&schema, &json!(8.0)), None);
    assert_eq!(
        pointer_of_violation(&schema, &json!(8.5)),
        Some(String::new())
    );
}

#[test]
fn conditional_and_reference_keywords() {
    let schema = json!({
        "$defs": {"name": {"type": "string", "minLength": 2}},
        "properties": {"name": {"$ref": "#/$defs/name"}, "kind": {}},
        "if": {"properties": {"kind": {"const": "a"}}},
        "then": {"required": ["name"]},
        "else": {"properties": {"name": false}}
    });
    assert_eq!(
        pointer_of_violation(&schema, &json!({"kind": "a", "name": "ok"})),
        None
    );
    assert_eq!(
        pointer_of_violation(&schema, &json!({"kind": "a"})),
        Some(String::new())
    );
    assert_eq!(
        pointer_of_violation(&schema, &json!({"kind": "a", "name": "x"})),
        Some("/name".to_owned())
    );
    assert_eq!(
        pointer_of_violation(&schema, &json!({"kind": "b", "name": "ok"})),
        Some("/name".to_owned())
    );
}

#[test]
fn unsupported_keywords_patterns_and_references_fail_closed() {
    let unknown_keyword = json!({"multipleOf": 2});
    assert_eq!(
        pointer_of_violation(&unknown_keyword, &json!(4)),
        Some(String::new())
    );
    let unknown_pattern = json!({"pattern": "^a+$"});
    assert_eq!(
        pointer_of_violation(&unknown_pattern, &json!("a")),
        Some(String::new())
    );
    let unknown_reference = json!({"$ref": "https://example.invalid/schema"});
    assert_eq!(
        pointer_of_violation(&unknown_reference, &json!(1)),
        Some(String::new())
    );
}

#[test]
fn pointers_escape_member_names() {
    let schema = json!({"additionalProperties": false});
    assert_eq!(
        pointer_of_violation(&schema, &json!({"a/b~c": 1})),
        Some("/a~1b~0c".to_owned())
    );
}

#[test]
fn every_violation_is_collected_and_the_first_is_the_one_validate_returns() {
    let schema = json!({
        "required": ["a", "b"],
        "properties": {"x": {"type": "string", "enum": ["y"]}, "list": {"minItems": 2}},
        "additionalProperties": false
    });
    let instance = json!({"x": 1, "list": [], "zz": true});
    let found = Evaluator::new(&schema).violations(&schema, &instance, "");
    let summary: Vec<(&str, &SchemaViolationKind)> = found
        .iter()
        .map(|violation| (violation.pointer.as_str(), &violation.kind))
        .collect();
    assert_eq!(
        summary,
        [
            ("", &SchemaViolationKind::Required("a".to_owned())),
            ("", &SchemaViolationKind::Required("b".to_owned())),
            ("/list", &SchemaViolationKind::MinItems),
            ("/x", &SchemaViolationKind::Type),
            ("/x", &SchemaViolationKind::Enum),
            ("/zz", &SchemaViolationKind::AdditionalProperties),
        ]
    );
    let first = Evaluator::new(&schema).validate(&schema, &instance, "");
    assert_eq!(first, Err(found[0].clone()));
}

/// Every keyword of every published schema is implemented or a pure annotation.
#[test]
fn every_schema_keyword_is_supported() {
    for kind in ContractKind::ALL {
        let schema: Value = serde_json::from_str(schema_text(kind)).expect("schema is JSON");
        let mut found = Vec::new();
        keywords(&schema, &mut found);
        for name in found {
            assert!(
                ASSERTION_KEYWORDS.contains(&name.as_str())
                    || ANNOTATION_KEYWORDS.contains(&name.as_str()),
                "{kind}: unsupported keyword {name}"
            );
        }
    }
}

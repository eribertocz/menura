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

/// OSS-5.1 needs the development-line schema evaluated unchanged: it refers to JSON Pointers
/// inside definitions (`#/$defs/SemanticModel/properties/name`).
#[test]
fn con_1_oss_5_1_a_reference_to_a_pointer_inside_a_definition_is_resolved() {
    let schema = json!({
        "$defs": {"Model": {"properties": {"name": {"type": "string", "minLength": 2}}}},
        "properties": {"name": {"$ref": "#/$defs/Model/properties/name"}}
    });
    assert_eq!(pointer_of_violation(&schema, &json!({"name": "ok"})), None);
    assert_eq!(
        pointer_of_violation(&schema, &json!({"name": "x"})),
        Some("/name".to_owned())
    );
    assert_eq!(
        pointer_of_violation(&schema, &json!({"name": 1})),
        Some("/name".to_owned())
    );
}

#[test]
fn con_1_reference_tokens_are_unescaped_as_json_pointers() {
    let schema = json!({
        "$defs": {"a/b": {"properties": {"x": {"type": "integer"}}}, "c~d": {"type": "integer"}},
        "properties": {
            "slash": {"$ref": "#/$defs/a~1b/properties/x"},
            "tilde": {"$ref": "#/$defs/c~0d"}
        }
    });
    assert_eq!(
        pointer_of_violation(&schema, &json!({"slash": 1, "tilde": 2})),
        None
    );
    assert_eq!(
        pointer_of_violation(&schema, &json!({"slash": "x"})),
        Some("/slash".to_owned())
    );
    assert_eq!(
        pointer_of_violation(&schema, &json!({"tilde": "x"})),
        Some("/tilde".to_owned())
    );
}

#[test]
fn con_1_references_outside_definitions_or_unresolvable_fail_closed() {
    for reference in [
        "#/properties/name",
        "#",
        "#/$defs/Missing/properties/name",
        "#/$defs/Model/properties/missing",
        "#/$defs/Model%2Fname",
        "other.json#/$defs/Model",
    ] {
        let schema = json!({
            "$defs": {"Model": {"properties": {"name": {"type": "string"}}}},
            "properties": {"name": {"$ref": reference}}
        });
        let violation = Evaluator::new(&schema)
            .validate(&schema, &json!({"name": "ok"}), "")
            .expect_err(reference);
        assert_eq!(violation.kind, SchemaViolationKind::UnsupportedSchema);
        assert_eq!(violation.pointer, "/name");
    }
}

/// The annotation keywords of JSON Schema 2020-12 assert nothing, whatever their value.
#[test]
fn con_1_oss_5_1_annotation_keywords_are_ignored() {
    let schema = json!({
        "type": "object",
        "title": "Model",
        "description": "A model.",
        "$comment": "Annotations only.",
        "default": {},
        "deprecated": true,
        "readOnly": true,
        "writeOnly": false,
        "examples": [{"name": 1}],
        "properties": {
            "name": {"type": "string", "examples": ["shop"], "deprecated": false},
            "examples": {"type": "array"}
        }
    });
    assert_eq!(
        pointer_of_violation(&schema, &json!({"name": "shop", "examples": []})),
        None
    );
    assert_eq!(
        pointer_of_violation(&schema, &json!({"name": 1})),
        Some("/name".to_owned())
    );
    assert_eq!(
        pointer_of_violation(&schema, &json!({"examples": {}})),
        Some("/examples".to_owned())
    );
}

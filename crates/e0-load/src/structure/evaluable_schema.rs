//! Rewrites an embedded model schema into an equivalent one that the JSON Schema evaluator of
//! `semantic-compiler-contracts` evaluates.
//!
//! The evaluator resolves `$ref` only to a whole definition (`#/$defs/<name>`) and fails closed
//! on any keyword it does not list. The development-line schema (OSS-5.1) also uses `$ref` to a
//! JSON Pointer inside a definition (`#/$defs/SemanticModel/properties/name`) and the annotation
//! `examples`. Both rewrites keep the meaning of JSON Schema 2020-12:
//!
//! - a `$ref` to another place of the schema becomes `allOf` of the subschema it names, which
//!   applies it to the same instance, as `$ref` does;
//! - `examples` asserts nothing and is removed.
//!
//! The embedded file itself is never changed: it stays byte for byte as published.

use serde_json::{Map, Value as Json};

/// The `$ref` prefix the evaluator resolves itself.
const DEFINITION_PREFIX: &str = "#/$defs/";

/// Annotation keywords of JSON Schema 2020-12 that the evaluator does not list.
const UNLISTED_ANNOTATIONS: [&str; 1] = ["examples"];

/// How deep subschemas are rewritten. A `$ref` that is still deeper is left as it is, and the
/// evaluator fails closed on it.
const MAXIMUM_DEPTH: usize = 64;

/// The evaluable form of the schema `text`.
///
/// # Errors
///
/// Returns the JSON error when `text` is not JSON.
pub(crate) fn evaluable_schema(text: &str) -> Result<Json, serde_json::Error> {
    let root: Json = serde_json::from_str(text)?;
    let mut schema = root.clone();
    rewrite(&root, &mut schema, 0);
    Ok(schema)
}

/// Rewrites `schema` and every subschema inside it.
fn rewrite(root: &Json, schema: &mut Json, depth: usize) {
    let Json::Object(keywords) = schema else {
        return;
    };
    if depth > MAXIMUM_DEPTH {
        return;
    }
    for name in UNLISTED_ANNOTATIONS {
        keywords.remove(name);
    }
    inline_reference(root, keywords);
    for (name, member) in keywords.iter_mut() {
        match name.as_str() {
            "properties" | "$defs" => {
                if let Json::Object(subschemas) = member {
                    for subschema in subschemas.values_mut() {
                        rewrite(root, subschema, depth + 1);
                    }
                }
            }
            "allOf" | "anyOf" | "oneOf" => {
                if let Json::Array(subschemas) = member {
                    for subschema in subschemas {
                        rewrite(root, subschema, depth + 1);
                    }
                }
            }
            "items" | "additionalProperties" | "not" | "if" | "then" | "else" => {
                rewrite(root, member, depth + 1);
            }
            _ => {}
        }
    }
}

/// Replaces a `$ref` to a place other than a whole definition by `allOf` of that place.
fn inline_reference(root: &Json, keywords: &mut Map<String, Json>) {
    let Some(Json::String(reference)) = keywords.get("$ref") else {
        return;
    };
    if is_whole_definition(reference) {
        return;
    }
    let Some(target) = reference
        .strip_prefix('#')
        .and_then(|pointer| root.pointer(pointer))
    else {
        return;
    };
    let target = target.clone();
    keywords.remove("$ref");
    match keywords.get_mut("allOf") {
        Some(Json::Array(all)) => all.push(target),
        _ => {
            keywords.insert("allOf".to_owned(), Json::Array(vec![target]));
        }
    }
}

fn is_whole_definition(reference: &str) -> bool {
    reference
        .strip_prefix(DEFINITION_PREFIX)
        .is_some_and(|name| !name.contains('/'))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::evaluable_schema;

    #[test]
    fn a_reference_inside_a_definition_becomes_all_of() {
        let text = r##"{"properties": {"name": {"$ref": "#/$defs/Model/properties/name"}, "model": {"$ref": "#/$defs/Model"}}, "$defs": {"Model": {"properties": {"name": {"type": "string", "examples": ["a"]}}}}}"##;
        let schema = evaluable_schema(text).expect("the schema is JSON");
        assert_eq!(
            schema["properties"]["name"],
            json!({"allOf": [{"type": "string"}]})
        );
        assert_eq!(
            schema["properties"]["model"],
            json!({"$ref": "#/$defs/Model"})
        );
    }

    #[test]
    fn a_property_named_examples_is_kept() {
        let text = r#"{"properties": {"examples": {"type": "array"}}, "examples": [{}]}"#;
        let schema = evaluable_schema(text).expect("the schema is JSON");
        assert_eq!(
            schema,
            json!({"properties": {"examples": {"type": "array"}}})
        );
    }

    #[test]
    fn an_unresolvable_reference_is_left_for_the_evaluator() {
        let text = r##"{"$ref": "#/$defs/Missing/properties/name"}"##;
        let schema = evaluable_schema(text).expect("the schema is JSON");
        assert_eq!(schema, json!({"$ref": "#/$defs/Missing/properties/name"}));
    }
}

//! Validation against the schema of the model format (E0B-1, OSS-5.1), with each schema
//! violation turned into its E0B-4 diagnostic.

use menura_contracts::types::audit_record::OssieFormat;
use menura_contracts::{SchemaViolation, SchemaViolationKind, schema_violations};

use super::diagnostic::{StructureCode, StructureDiagnostic};
use super::json::to_json;
use super::schema::schema_text;
use crate::pointer::{JsonPointer, resolve};
use crate::read::Node;

/// Every diagnostic of `document` against the schema of `format`, unordered: the official 0.1.1
/// schema, or the development-line schema (MDL-1.1).
pub(crate) fn schema_diagnostics(document: &Node, format: OssieFormat) -> Vec<StructureDiagnostic> {
    match serde_json::from_str::<serde_json::Value>(schema_text(format)) {
        Ok(schema) => schema_violations(&schema, &to_json(document))
            .into_iter()
            .map(|violation| diagnostic(document, violation))
            .collect(),
        Err(error) => vec![StructureDiagnostic {
            code: StructureCode::WrongType,
            pointer: JsonPointer::root(),
            location: document.location,
            message: format!("the embedded model schema is not valid JSON: {error}"),
        }],
    }
}

fn diagnostic(document: &Node, violation: SchemaViolation) -> StructureDiagnostic {
    let pointer = JsonPointer::from_text(&violation.pointer);
    let location = resolve(document, &pointer).map_or(document.location, |node| node.location);
    StructureDiagnostic {
        code: code_of(&pointer, &violation.kind),
        pointer,
        location,
        message: violation.message,
    }
}

/// The E0B-4 code of a violation. Any problem with `version` is `E0B-01`.
///
/// The official schema uses only `type`, `const`, `enum`, `required`, `properties`,
/// `additionalProperties`, `items`, `minItems`, `oneOf` and `$ref`; the development-line schema
/// adds `minLength` and `anyOf` (checked by tests). The only `oneOf` of both chooses between a
/// string and an object (`AIContext`), so failing it is a wrong type. The only `anyOf`, of the
/// development line, requires `sql` or `dialects` (`Expression`), so failing it is a missing
/// property.
fn code_of(pointer: &JsonPointer, kind: &SchemaViolationKind) -> StructureCode {
    let version = JsonPointer::root().member("version");
    match kind {
        SchemaViolationKind::Required(member)
            if member == "version" && pointer.as_str().is_empty() =>
        {
            StructureCode::Version
        }
        _ if *pointer == version => StructureCode::Version,
        SchemaViolationKind::Required(_) | SchemaViolationKind::AnyOf => {
            StructureCode::MissingProperty
        }
        SchemaViolationKind::AdditionalProperties => StructureCode::PropertyNotAllowed,
        SchemaViolationKind::Enum | SchemaViolationKind::Const => StructureCode::OutsideEnumeration,
        SchemaViolationKind::MinItems => StructureCode::EmptyList,
        SchemaViolationKind::Type
        | SchemaViolationKind::OneOf
        | SchemaViolationKind::Not
        | SchemaViolationKind::FalseSchema
        | SchemaViolationKind::Minimum
        | SchemaViolationKind::Maximum
        | SchemaViolationKind::MinLength
        | SchemaViolationKind::MaxLength
        | SchemaViolationKind::Pattern
        | SchemaViolationKind::Format
        | SchemaViolationKind::MaxItems
        | SchemaViolationKind::UniqueItems
        | SchemaViolationKind::UnsupportedSchema => StructureCode::WrongType,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value as Json;

    use crate::structure::schema::{DEVELOPMENT_SCHEMA_TEXT, OFFICIAL_SCHEMA_TEXT};

    /// The keywords `code_of` maps, plus annotations.
    const MAPPED: [&str; 15] = [
        "type",
        "const",
        "enum",
        "required",
        "properties",
        "additionalProperties",
        "items",
        "minItems",
        "oneOf",
        "$ref",
        "$schema",
        "$id",
        "$defs",
        "title",
        "description",
    ];

    fn keywords(schema: &Json, found: &mut Vec<String>) {
        let Json::Object(members) = schema else {
            return;
        };
        for (name, member) in members {
            found.push(name.clone());
            match name.as_str() {
                "properties" | "$defs" => member
                    .as_object()
                    .into_iter()
                    .flat_map(|map| map.values())
                    .for_each(|subschema| keywords(subschema, found)),
                "oneOf" | "anyOf" => member
                    .as_array()
                    .into_iter()
                    .flatten()
                    .for_each(|subschema| keywords(subschema, found)),
                "items" | "additionalProperties" => keywords(member, found),
                _ => {}
            }
        }
    }

    /// The keywords the development-line schema adds, and the annotation `examples`.
    const DEVELOPMENT_ONLY: [&str; 3] = ["minLength", "anyOf", "examples"];

    fn schema_keywords(text: &str) -> Vec<String> {
        let schema: Json = serde_json::from_str(text).expect("the schema is JSON");
        let mut found = Vec::new();
        keywords(&schema, &mut found);
        found
    }

    #[test]
    fn e0b_1_1_the_official_schema_uses_only_mapped_keywords() {
        for name in schema_keywords(OFFICIAL_SCHEMA_TEXT) {
            assert!(MAPPED.contains(&name.as_str()), "unmapped keyword {name}");
        }
    }

    #[test]
    fn oss_5_1_the_development_schema_uses_only_mapped_keywords() {
        for name in schema_keywords(DEVELOPMENT_SCHEMA_TEXT) {
            assert!(
                MAPPED.contains(&name.as_str()) || DEVELOPMENT_ONLY.contains(&name.as_str()),
                "unmapped keyword {name}"
            );
        }
    }

    #[test]
    fn oss_5_1_every_any_of_of_the_development_schema_only_requires_members() {
        let schema: Json =
            serde_json::from_str(DEVELOPMENT_SCHEMA_TEXT).expect("the schema is JSON");
        let alternatives = schema["$defs"]["Expression"]["anyOf"]
            .as_array()
            .expect("Expression has anyOf");
        for alternative in alternatives {
            let members = alternative
                .as_object()
                .expect("an alternative is an object");
            assert!(members.keys().all(|name| name == "required"));
        }
        let count = DEVELOPMENT_SCHEMA_TEXT.matches("\"anyOf\"").count();
        assert_eq!(count, 1, "the development schema has one anyOf");
    }
}

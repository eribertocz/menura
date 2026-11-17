//! Validation against the official schema (E0B-1), with each schema violation turned into its
//! E0B-4 diagnostic.

use semantic_compiler_contracts::{SchemaViolation, SchemaViolationKind, schema_violations};
use serde_json::Value as Json;

use super::diagnostic::{StructureCode, StructureDiagnostic};
use super::json::to_json;
use super::schema::OFFICIAL_SCHEMA_TEXT;
use crate::pointer::{JsonPointer, resolve};
use crate::read::Node;

/// Every diagnostic of `document` against the official 0.1.1 schema, unordered.
pub(crate) fn official_schema_diagnostics(document: &Node) -> Vec<StructureDiagnostic> {
    match serde_json::from_str::<Json>(OFFICIAL_SCHEMA_TEXT) {
        Ok(schema) => schema_violations(&schema, &to_json(document))
            .into_iter()
            .map(|violation| diagnostic(document, violation))
            .collect(),
        Err(error) => vec![StructureDiagnostic {
            code: StructureCode::WrongType,
            pointer: JsonPointer::root(),
            location: document.location,
            message: format!("the embedded official schema is not valid JSON: {error}"),
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
/// `additionalProperties`, `items`, `minItems`, `oneOf` and `$ref` (checked by a test). Its only
/// `oneOf` chooses between a string and an object (`AIContext`), so failing it is a wrong type.
fn code_of(pointer: &JsonPointer, kind: &SchemaViolationKind) -> StructureCode {
    let version = JsonPointer::root().member("version");
    match kind {
        SchemaViolationKind::Required(member)
            if member == "version" && pointer.as_str().is_empty() =>
        {
            StructureCode::Version
        }
        _ if *pointer == version => StructureCode::Version,
        SchemaViolationKind::Required(_) => StructureCode::MissingProperty,
        SchemaViolationKind::AdditionalProperties => StructureCode::PropertyNotAllowed,
        SchemaViolationKind::Enum | SchemaViolationKind::Const => StructureCode::OutsideEnumeration,
        SchemaViolationKind::MinItems => StructureCode::EmptyList,
        SchemaViolationKind::Type
        | SchemaViolationKind::OneOf
        | SchemaViolationKind::AnyOf
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

    use super::OFFICIAL_SCHEMA_TEXT;

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
                "oneOf" => member
                    .as_array()
                    .into_iter()
                    .flatten()
                    .for_each(|subschema| keywords(subschema, found)),
                "items" | "additionalProperties" => keywords(member, found),
                _ => {}
            }
        }
    }

    #[test]
    fn e0b_1_1_the_official_schema_uses_only_mapped_keywords() {
        let schema: Json = serde_json::from_str(OFFICIAL_SCHEMA_TEXT).expect("the schema is JSON");
        let mut found = Vec::new();
        keywords(&schema, &mut found);
        for name in found {
            assert!(MAPPED.contains(&name.as_str()), "unmapped keyword {name}");
        }
    }
}

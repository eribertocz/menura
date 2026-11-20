//! Names, source locations and descriptions of the elements, read from the data tree
//! (E0H-1.5, E0H-2 "Descriptions").

use menura_model::internal_model::{Descriptions, NormalizedName, SourceLocation};
use menura_model::json::JsonForm;

use crate::names::Name;
use crate::nodes::{member, text};
use crate::outline::DeclaredName;
use crate::pointer::{JsonPointer, key_text, resolve};
use crate::read::{Location, Node, Value};

/// The largest integer written as a JSON number (CON-4).
const MAXIMUM_EXACT_NUMBER: u64 = (1 << 53) - 1;

/// The model's form of a normalized name.
pub(super) fn normalized(name: &Name) -> NormalizedName {
    NormalizedName::new(name.as_str())
}

/// The normalized form of a declared name, or the empty name when it is not written; the
/// structure is valid, so every required name is written.
pub(super) fn declared(name: Option<&DeclaredName<'_>>) -> NormalizedName {
    name.map_or_else(|| NormalizedName::new(""), |name| normalized(&name.name))
}

/// The normalized form of a name written as a string of the model document (E0D-1.4).
pub(super) fn written(text: &str) -> NormalizedName {
    normalized(&Name::declared(text))
}

/// The location of `pointer`, which starts at `location`.
pub(super) fn located(pointer: &JsonPointer, location: Location) -> SourceLocation {
    SourceLocation {
        pointer: pointer.as_str().to_owned(),
        line: location.line,
        column: location.column,
    }
}

/// The location of the node `pointer` names in `document`.
pub(super) fn location_of(document: &Node, pointer: &JsonPointer) -> SourceLocation {
    let location = resolve(document, pointer).map_or(document.location, |node| node.location);
    located(pointer, location)
}

/// The `description`, `label` and `ai_context` of the node `pointer` names, as written.
pub(super) fn descriptions(document: &Node, pointer: &JsonPointer) -> Descriptions {
    let Some(node) = resolve(document, pointer) else {
        return Descriptions::default();
    };
    Descriptions {
        description: text(member(node, "description")).map(str::to_owned),
        label: text(member(node, "label")).map(str::to_owned),
        ai_context: member(node, "ai_context").map(json_form),
    }
}

/// The JSON form of a node of the data tree. A key that is not a string becomes its JSON text,
/// and the first of two keys with the same text is kept, as JSON Pointers resolve them.
fn json_form(node: &Node) -> JsonForm {
    match &node.value {
        Value::Null => JsonForm::Null,
        Value::Boolean(value) => JsonForm::Boolean(*value),
        Value::Integer(value) => match value.to_string().parse::<i64>() {
            Ok(number) if number.unsigned_abs() <= MAXIMUM_EXACT_NUMBER => {
                JsonForm::Integer(number)
            }
            _ => JsonForm::Text(value.to_string()),
        },
        Value::Float(value) => JsonForm::Number(format!("{value:e}")),
        Value::String(text) => JsonForm::text(text),
        Value::Sequence(items) => JsonForm::list(items.iter(), json_form),
        Value::Mapping(entries) => {
            let mut members = std::collections::BTreeMap::new();
            for entry in entries.iter() {
                members
                    .entry(key_text(&entry.key))
                    .or_insert_with(|| json_form(&entry.value));
            }
            JsonForm::Object(members)
        }
    }
}

/// A positive integer of an extension, as CON-4 writes it: a JSON number within ±(2^53 − 1),
/// otherwise its canonical string.
pub(super) fn normalized_number(value: u64) -> serde_json::Value {
    if value <= MAXIMUM_EXACT_NUMBER {
        serde_json::Value::from(value)
    } else {
        serde_json::Value::String(value.to_string())
    }
}

//! The reading of a model document by E0.a and E0.b alone, without the model selection of
//! E0B-3.2 (MIG-3.1): the entry point of `migrate`.

use menura_contracts::types::audit_record::OssieFormat;
use menura_contracts::types::compiled_plan::Diagnostic;
use menura_contracts::types::ordered_json::{JsonInteger, OrderedJson, OrderedObject};

use super::LoadRejection;
use super::findings::{self, Substage};
use super::report::{Report, model_location};
use crate::pointer::{JsonPointer, key_text};
use crate::read::{Location, Node, Value, read_document};
use crate::structure::schema_errors;

/// The code of a number of the model that has no JSON form (MIG-3.3).
const NO_JSON_FORM: &str = "EMIG-01";

/// Reads the model document `model` with E0.a and validates its structure with E0.b in the
/// format `format` (section 4.5), covering every model of the document: no model is selected
/// (MIG-3.1). Returns the document tree E0.a produces, anchors and aliases expanded and dates as
/// text, as a JSON object whose members keep the order they were read in and whose integers
/// keep their exact digits (MIG-3.3).
///
/// Keys that are not strings become their JSON text; of two keys with the same text the first
/// is kept, as JSON Pointers resolve them.
///
/// # Errors
///
/// A [`LoadRejection`] without warnings: the errors of E0.a or E0.b when either rejects the
/// document (MIG-3.1); otherwise `EMIG-01` at every number without a JSON form (`.inf`,
/// `.nan`), in document order (MIG-3.3).
///
/// # Example
///
/// ```
/// use menura_contracts::types::audit_record::OssieFormat;
/// use menura_e0_load::read_model_tree;
///
/// let rejection = read_model_tree(b"version: [", OssieFormat::Release).unwrap_err();
/// assert_eq!(rejection.errors[0].code, "E0A-02");
/// ```
pub fn read_model_tree(model: &[u8], format: OssieFormat) -> Result<OrderedObject, LoadRejection> {
    let mut report = Report::default();
    let document = match read_document(model) {
        Ok(document) => document,
        Err(error) => {
            report.add_read_error(&error);
            return Err(rejection(report));
        }
    };
    let mut substage = Substage::default();
    substage.extend(
        schema_errors(&document, format)
            .iter()
            .map(findings::structure),
    );
    if report.add_substage(&document, substage) {
        return Err(rejection(report));
    }
    let mut without_form = Vec::new();
    let tree = ordered_json(&document, &JsonPointer::root(), &mut without_form);
    for (pointer, location) in &without_form {
        report.add_error(Diagnostic {
            code: NO_JSON_FORM.to_owned(),
            message: "a number of the model has no JSON form".to_owned(),
            location: Some(model_location(
                &pointer.to_string(),
                location.line,
                location.column,
            )),
            alternatives: None,
        });
    }
    match tree {
        OrderedJson::Object(object) if without_form.is_empty() => Ok(object),
        // Not an object: unreachable, since both schemas require an object at the root.
        _ => Err(rejection(report)),
    }
}

/// The rejection of `report`, without its warnings (MIG-3.1).
fn rejection(report: Report) -> LoadRejection {
    LoadRejection {
        warnings: Vec::new(),
        ..report.into_rejection()
    }
}

/// The JSON value of `node` at `pointer`; every number without a JSON form is added to
/// `without_form`, in document order, and stands as `null`.
fn ordered_json(
    node: &Node,
    pointer: &JsonPointer,
    without_form: &mut Vec<(JsonPointer, Location)>,
) -> OrderedJson {
    let integer = match &node.value {
        Value::Integer(value) => Some(JsonInteger::parse(&value.to_string())),
        _ => None,
    };
    match (&node.value, integer) {
        (_, Some(Some(integer))) => OrderedJson::Integer(integer),
        (Value::Boolean(value), _) => OrderedJson::Boolean(*value),
        (Value::Float(value), _) if value.is_finite() => OrderedJson::Float(*value),
        (Value::String(text), _) => OrderedJson::String(text.clone()),
        (Value::Sequence(items), _) => OrderedJson::Array(
            items
                .iter()
                .enumerate()
                .map(|(index, item)| ordered_json(item, &pointer.item(index), without_form))
                .collect(),
        ),
        (Value::Mapping(entries), _) => {
            let mut members: Vec<(String, OrderedJson)> = Vec::with_capacity(entries.len());
            for entry in entries.iter() {
                let name = key_text(&entry.key);
                if members.iter().all(|(existing, _)| *existing != name) {
                    let value = ordered_json(&entry.value, &pointer.member(&name), without_form);
                    members.push((name, value));
                }
            }
            OrderedJson::Object(OrderedObject::new(members))
        }
        (Value::Null, _) => OrderedJson::Null,
        (Value::Float(_) | Value::Integer(_), _) => {
            without_form.push((pointer.clone(), node.location));
            OrderedJson::Null
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(text: &str) -> String {
        let object = read_model_tree(text.as_bytes(), OssieFormat::Release).unwrap();
        serde_json::to_string(&object).unwrap()
    }

    const DATASET: &str = "[{name: d, source: s.t}]";

    fn models() -> String {
        format!(
            "version: \"0.1.1\"\nsemantic_model:\n  - name: b\n    datasets: {DATASET}\n  - name: a\n    datasets: {DATASET}\n"
        )
    }

    #[test]
    fn mig_3_1_every_model_is_read_without_a_selection() {
        assert_eq!(
            tree(&models()),
            r#"{"version":"0.1.1","semantic_model":[{"name":"b","datasets":[{"name":"d","source":"s.t"}]},{"name":"a","datasets":[{"name":"d","source":"s.t"}]}]}"#
        );
    }

    #[test]
    fn mig_3_1_a_structure_error_is_the_compile_error_without_warnings() {
        let rejection = read_model_tree(b"version: \"0.1.1\"\n", OssieFormat::Release).unwrap_err();
        assert!(rejection.errors[0].code.starts_with("E0B-"));
        assert!(rejection.warnings.is_empty());
    }

    #[test]
    fn mig_3_3_aliases_are_expanded_and_dates_are_text() {
        let text = format!(
            "version: \"0.1.1\"\nsemantic_model:\n  - name: m\n    description: &d 2026-01-02\n    datasets: {DATASET}\n    ai_context: *d\n"
        );
        assert_eq!(
            tree(&text),
            r#"{"version":"0.1.1","semantic_model":[{"name":"m","description":"2026-01-02","datasets":[{"name":"d","source":"s.t"}],"ai_context":"2026-01-02"}]}"#
        );
    }

    #[test]
    fn mig_3_3_integers_keep_their_exact_digits() {
        let document =
            read_document(b"[1, -2, 1.5, 0x10, 123456789012345678901234567890123456789012]")
                .unwrap();
        let mut without_form = Vec::new();
        let OrderedJson::Array(items) =
            ordered_json(&document, &JsonPointer::root(), &mut without_form)
        else {
            panic!("an array");
        };
        let texts: Vec<String> = items
            .iter()
            .map(|item| match item {
                OrderedJson::Integer(integer) => integer.as_str().to_owned(),
                other => serde_json::to_string(other).unwrap(),
            })
            .collect();
        assert_eq!(
            texts,
            [
                "1",
                "-2",
                "1.5",
                "16",
                "123456789012345678901234567890123456789012"
            ]
        );
        assert!(without_form.is_empty());
    }

    #[test]
    fn mig_3_3_a_number_without_a_json_form_is_emig_01_at_that_value() {
        let text = "version: 0.3.0.dev0\nname: m\ndatasets:\n  - name: d\n    source: s.t\n    nonportable_extensions:\n      - vendor_name: COMMON\n        data: {limits: [.nan, .inf]}\n";
        let errors = read_model_tree(text.as_bytes(), OssieFormat::Development)
            .unwrap_err()
            .errors;
        let found: Vec<(&str, &str)> = errors
            .iter()
            .map(|error| {
                let location = error.location.as_ref().unwrap();
                (error.code.as_str(), location.pointer.as_str())
            })
            .collect();
        assert_eq!(
            found,
            [
                (
                    "EMIG-01",
                    "/datasets/0/nonportable_extensions/0/data/limits/0"
                ),
                (
                    "EMIG-01",
                    "/datasets/0/nonportable_extensions/0/data/limits/1"
                ),
            ]
        );
        assert_eq!(errors[0].location.as_ref().unwrap().line, Some(8));
    }
}

//! The reading of a model document by E0.a and E0.b alone, without the model selection of
//! E0B-3.2 (MIG-3.1): the entry point of `migrate`.

use menura_contracts::types::audit_record::OssieFormat;
use menura_contracts::types::ordered_json::{OrderedJson, OrderedObject};

use super::LoadRejection;
use super::findings::{self, Substage};
use super::report::Report;
use crate::pointer::key_text;
use crate::read::{Node, Value, read_document};
use crate::structure::schema_errors;

/// Reads the model document `model` with E0.a and validates its structure with E0.b in the
/// format `format` (section 4.5), covering every model of the document: no model is selected
/// (MIG-3.1). Returns the document tree E0.a produces, anchors and aliases expanded and dates as
/// text, as a JSON object whose members keep the order they were read in (MIG-3.3).
///
/// Keys that are not strings become their JSON text; of two keys with the same text the first
/// is kept, as JSON Pointers resolve them. An integer beyond 128 bits becomes the nearest
/// number, and `.inf` and `.nan`, which have no JSON form, become `null`.
///
/// # Errors
///
/// A [`LoadRejection`] with the errors of E0.a or E0.b, and no warning, when either rejects
/// the document.
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
    match ordered_json(&document) {
        OrderedJson::Object(object) => Ok(object),
        // Unreachable: both schemas require an object at the root.
        _ => Ok(OrderedObject::default()),
    }
}

/// The rejection of `report`, without its warnings (MIG-3.1).
fn rejection(report: Report) -> LoadRejection {
    LoadRejection {
        warnings: Vec::new(),
        ..report.into_rejection()
    }
}

/// The JSON value of `node`.
fn ordered_json(node: &Node) -> OrderedJson {
    match &node.value {
        Value::Boolean(value) => OrderedJson::Boolean(*value),
        Value::Integer(value) => integer(&value.to_string()),
        Value::Float(value) if value.is_finite() => OrderedJson::Float(*value),
        // `.inf` and `.nan` have no JSON form.
        Value::Null | Value::Float(_) => OrderedJson::Null,
        Value::String(text) => OrderedJson::String(text.clone()),
        Value::Sequence(items) => OrderedJson::Array(items.iter().map(ordered_json).collect()),
        Value::Mapping(entries) => {
            let mut members: Vec<(String, OrderedJson)> = Vec::with_capacity(entries.len());
            for entry in entries.iter() {
                let name = key_text(&entry.key);
                if members.iter().all(|(existing, _)| *existing != name) {
                    members.push((name, ordered_json(&entry.value)));
                }
            }
            OrderedJson::Object(OrderedObject::new(members))
        }
    }
}

/// The JSON value of the integer written `digits`: exact within 128 bits, the nearest number
/// beyond.
fn integer(digits: &str) -> OrderedJson {
    match digits.parse::<i128>() {
        Ok(value) => OrderedJson::Integer(value),
        Err(_) => digits
            .parse::<f64>()
            .map_or(OrderedJson::Null, OrderedJson::Float),
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
    fn numbers_keep_their_value_when_json_has_it() {
        let document =
            read_document(b"[1, -2, 1.5, 123456789012345678901234567890, .inf]").unwrap();
        assert_eq!(
            serde_json::to_string(&ordered_json(&document)).unwrap(),
            "[1,-2,1.5,123456789012345678901234567890,null]"
        );
    }
}

//! What the specification defines from the text of an input, so that reordering the keys of
//! that input legitimately changes it (7.1a, Determinism):
//!
//! - `line` and `column` of a location, the position of the text in its document;
//! - the document order of diagnostics (E0-4.3) and of migration changes (MIG-3.4);
//! - the member order of the migrated model (MIG-3.3).
//!
//! A document that is not byte-identical is compared as data apart from the parts above, so
//! that every other value (`sql`, `parameters`, `cache_key`, every hash) must be identical.

use serde_json::Value;

/// The contract of the document `migrate` returns (MIG-3).
const MIGRATION_CONTRACT: &str = "menura.migration_report";

/// The lists of diagnostics, in document order (E0-4.3).
const DIAGNOSTIC_LISTS: [&str; 2] = ["errors", "warnings"];

/// The list of migration changes, in document order (MIG-3.4).
const MIGRATION_CHANGES: &str = "changes";

/// The members of a location that give a position in the text of its document.
const TEXT_POSITIONS: [&str; 2] = ["line", "column"];

/// Whether two output documents of the same call are equal once the parts defined from the
/// input text are set aside. A text that is not JSON is never equal to another text.
#[must_use]
pub fn equal_up_to_input_order(left: &str, right: &str) -> bool {
    let read = |text: &str| serde_json::from_str::<Value>(text).ok();
    let (Some(left_value), Some(right_value)) = (read(left), read(right)) else {
        return false;
    };
    let contract = left_value.get("contract").and_then(Value::as_str);
    if contract != right_value.get("contract").and_then(Value::as_str) {
        return false;
    }
    let migration = contract == Some(MIGRATION_CONTRACT);
    normalize(left_value, migration) == normalize(right_value, migration)
}

/// `value` as data, with positions removed from every location and the lists in document order
/// sorted; `changes` is sorted only in a migration report. Comparing two such values ignores
/// member order, which only the migrated model takes from its input.
fn normalize(value: Value, migration: bool) -> Value {
    match value {
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|item| normalize(item, migration))
                .collect(),
        ),
        Value::Object(entries) => Value::Object(
            entries
                .into_iter()
                .map(|(key, value)| {
                    let value = set_aside(&key, normalize(value, migration), migration);
                    (key, value)
                })
                .collect(),
        ),
        other => other,
    }
}

/// The member `key` with what it takes from the input text set aside.
fn set_aside(key: &str, value: Value, migration: bool) -> Value {
    let document_order = DIAGNOSTIC_LISTS.contains(&key) || (migration && key == MIGRATION_CHANGES);
    match value {
        Value::Object(mut location) if key == "location" => {
            for position in TEXT_POSITIONS {
                location.remove(position);
            }
            Value::Object(location)
        }
        Value::Array(mut items) if document_order => {
            items.sort_by_cached_key(Value::to_string);
            Value::Array(items)
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pln_2_4_positions_and_document_order_are_set_aside() {
        let left = r#"{"contract": "menura.compile_error", "errors": [
            {"code": "A", "location": {"pointer": "/a", "line": 2}},
            {"code": "B", "location": null}]}"#;
        let right = r#"{"errors": [{"code": "B", "location": null},
            {"code": "A", "location": {"line": 9, "pointer": "/a"}}],
            "contract": "menura.compile_error"}"#;
        assert!(equal_up_to_input_order(left, right));
    }

    #[test]
    fn pln_2_4_a_migrated_model_keeps_its_data() {
        let left = r#"{"contract": "menura.migration_report", "changes": [{"p": 1}, {"p": 2}],
            "model": {"x": 1, "y": 2}}"#;
        let right = r#"{"contract": "menura.migration_report", "changes": [{"p": 2}, {"p": 1}],
            "model": {"y": 2, "x": 1}}"#;
        assert!(equal_up_to_input_order(left, right));
        let changed = right.replace("\"x\": 1", "\"x\": 3");
        assert!(!equal_up_to_input_order(left, &changed));
    }

    #[test]
    fn pln_2_4_changes_keep_their_order_outside_a_migration_report() {
        let left = r#"{"contract": "menura.model_diff", "changes": [1, 2]}"#;
        let right = r#"{"contract": "menura.model_diff", "changes": [2, 1]}"#;
        assert!(!equal_up_to_input_order(left, right));
    }

    #[test]
    fn pln_2_4_a_plan_keeps_its_sql_and_hashes() {
        let plan = r#"{"contract": "menura.compiled_plan", "sql": "SELECT 1", "cache_key": "k",
            "warnings": [{"code": "W1"}, {"code": "W2"}]}"#;
        let warnings = plan.replace("\"W1\"}, {\"code\": \"W2\"", "\"W2\"}, {\"code\": \"W1\"");
        assert!(equal_up_to_input_order(plan, &warnings));
        assert!(!equal_up_to_input_order(
            plan,
            &plan.replace("SELECT 1", "SELECT  1")
        ));
        assert!(!equal_up_to_input_order(
            plan,
            &plan.replace("\"k\"", "\"j\"")
        ));
    }

    #[test]
    fn pln_2_4_any_other_difference_remains() {
        let left = r#"{"cache_key": "a", "location": {"pointer": "/a"}}"#;
        let other_key = r#"{"cache_key": "b", "location": {"pointer": "/a"}}"#;
        let other_pointer = r#"{"cache_key": "a", "location": {"pointer": "/b"}}"#;
        assert!(!equal_up_to_input_order(left, other_key));
        assert!(!equal_up_to_input_order(left, other_pointer));
        assert!(!equal_up_to_input_order(
            r#"{"sql": [1, 2]}"#,
            r#"{"sql": [2, 1]}"#
        ));
        assert!(!equal_up_to_input_order(
            "usage error: a",
            "usage error: a "
        ));
    }
}

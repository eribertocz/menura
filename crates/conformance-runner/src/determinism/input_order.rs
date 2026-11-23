//! What the specification defines from the text of an input, so that reordering the keys of
//! that input legitimately changes it (7.1a, Determinism):
//!
//! - `line` and `column` of a location, the position of the text in its document;
//! - the document order of E0 diagnostics (E0-4.3), wherever they are reported;
//! - in the report of `migrate`, the order of `changes` (MIG-3.4) and the member order of the
//!   migrated `model` (MIG-3.3).
//!
//! Plans and explanations stay byte for byte: every top-level member but `warnings` must have
//! the same text, and `warnings` may differ only by the exceptions above. The other documents
//! are compared as data with those exceptions; any other diagnostic keeps its position.

use serde_json::Value;

/// The contracts whose documents are plans (PLN-2.4, PLN-8.1).
const PLAN_CONTRACTS: [&str; 2] = ["menura.compiled_plan", "menura.explanation"];

/// The contract of the document `migrate` returns (MIG-3).
const MIGRATION_CONTRACT: &str = "menura.migration_report";

/// The members that hold diagnostics.
const DIAGNOSTIC_LISTS: [&str; 2] = ["errors", "warnings"];

/// The members of a location that give a position in the text of its document.
const TEXT_POSITIONS: [&str; 2] = ["line", "column"];

/// Whether two output documents of the same call are equal once what the specification
/// defines from the input text is set aside. A text that is not JSON is never equal to another
/// text.
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
    match contract {
        Some(plan) if PLAN_CONTRACTS.contains(&plan) => plans_equal(left, right),
        Some(MIGRATION_CONTRACT) => migration_data(left_value) == migration_data(right_value),
        _ => normalize(left_value) == normalize(right_value),
    }
}

/// Whether two plan documents have the same top-level members in the same order, each with the
/// same text, except `warnings`, whose diagnostics are compared as data.
fn plans_equal(left: &str, right: &str) -> bool {
    let (Some(left), Some(right)) = (top_level_members(left), top_level_members(right)) else {
        return false;
    };
    left.len() == right.len()
        && left.iter().zip(&right).all(|(left, right)| {
            left.0 == right.0
                && (left.1 == right.1
                    || (left.0 == "warnings" && same_member_data(&left.1, &right.1)))
        })
}

/// The top-level members of a document written as `core` writes it, two spaces per level: the
/// name of each member and the text of its lines.
fn top_level_members(text: &str) -> Option<Vec<(String, String)>> {
    let inner = text.strip_prefix("{\n")?.strip_suffix("\n}")?;
    let mut members: Vec<(String, String)> = Vec::new();
    for line in inner.lines() {
        let starts_member = line.starts_with("  \"");
        if starts_member {
            let name = line.get(3..)?.split('"').next()?.to_owned();
            members.push((name, String::new()));
        }
        let (_, member) = members.last_mut()?;
        member.push_str(line);
        member.push('\n');
    }
    Some(members)
}

/// Whether two texts of the same member, `"name": value` with an optional final comma, hold
/// the same diagnostics once the exceptions are set aside.
fn same_member_data(left: &str, right: &str) -> bool {
    let value = |member: &str| {
        let (_, value) = member.split_once(": ")?;
        let value = value.trim_end().trim_end_matches(',');
        serde_json::from_str::<Value>(value)
            .ok()
            .map(|value| set_aside("warnings", normalize(value)))
    };
    matches!((value(left), value(right)), (Some(left), Some(right)) if left == right)
}

/// A migration report as data, with its `changes` sorted (MIG-3.4); comparing two such values
/// ignores the member order of the migrated model (MIG-3.3).
fn migration_data(value: Value) -> Value {
    match normalize(value) {
        Value::Object(mut report) => {
            if let Some(Value::Array(changes)) = report.get_mut("changes") {
                changes.sort_by_cached_key(Value::to_string);
            }
            Value::Object(report)
        }
        other => other,
    }
}

/// `value` as data, with positions removed from every location and E0 diagnostics sorted
/// among themselves.
fn normalize(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(normalize).collect()),
        Value::Object(entries) => Value::Object(
            entries
                .into_iter()
                .map(|(key, value)| {
                    let value = set_aside(&key, normalize(value));
                    (key, value)
                })
                .collect(),
        ),
        other => other,
    }
}

/// The member `key` with what it takes from the input text set aside.
fn set_aside(key: &str, value: Value) -> Value {
    match value {
        Value::Object(mut location) if key == "location" => {
            for position in TEXT_POSITIONS {
                location.remove(position);
            }
            Value::Object(location)
        }
        Value::Array(items) if DIAGNOSTIC_LISTS.contains(&key) => {
            Value::Array(sort_model_diagnostics(items))
        }
        other => other,
    }
}

/// The diagnostics with those of E0 sorted among themselves (E0-4.3) and every other one left
/// at its position.
fn sort_model_diagnostics(mut items: Vec<Value>) -> Vec<Value> {
    let positions: Vec<usize> = (0..items.len())
        .filter(|index| items.get(*index).is_some_and(is_model_diagnostic))
        .collect();
    let mut diagnostics: Vec<Value> = positions
        .iter()
        .filter_map(|index| items.get(*index).cloned())
        .collect();
    diagnostics.sort_by_cached_key(Value::to_string);
    for (index, diagnostic) in positions.into_iter().zip(diagnostics) {
        if let Some(slot) = items.get_mut(index) {
            *slot = diagnostic;
        }
    }
    items
}

/// Whether a diagnostic is one of E0, whose codes start with `E0` or `W0` (E0-4).
fn is_model_diagnostic(diagnostic: &Value) -> bool {
    diagnostic
        .get("code")
        .and_then(Value::as_str)
        .is_some_and(|code| code.starts_with("E0") || code.starts_with("W0"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pln_2_4_positions_and_e0_order_are_set_aside() {
        let left = r#"{"contract": "menura.validation_report", "errors": [
            {"code": "E0C-01", "location": {"pointer": "/a", "line": 2}},
            {"code": "E0B-02", "location": null}]}"#;
        let right = r#"{"errors": [{"code": "E0B-02", "location": null},
            {"code": "E0C-01", "location": {"line": 9, "pointer": "/a"}}],
            "contract": "menura.validation_report"}"#;
        assert!(equal_up_to_input_order(left, right));
    }

    #[test]
    fn pln_2_4_query_errors_keep_their_order() {
        let left = r#"{"contract": "menura.compile_error", "errors": [
            {"code": "E_NAME_NOT_FOUND"}, {"code": "E_NO_PATH"}]}"#;
        let right = r#"{"contract": "menura.compile_error", "errors": [
            {"code": "E_NO_PATH"}, {"code": "E_NAME_NOT_FOUND"}]}"#;
        assert!(!equal_up_to_input_order(left, right));
        let mixed = r#"{"contract": "menura.compile_error", "errors": [
            {"code": "E_NO_PATH"}, {"code": "W0C-01"}, {"code": "E0C-01"}]}"#;
        let swapped = r#"{"contract": "menura.compile_error", "errors": [
            {"code": "E0C-01"}, {"code": "W0C-01"}, {"code": "E_NO_PATH"}]}"#;
        assert!(!equal_up_to_input_order(mixed, swapped));
    }

    #[test]
    fn pln_2_4_only_a_migration_report_has_unordered_changes_and_model() {
        let left = r#"{"contract": "menura.migration_report", "changes": [{"p": 1}, {"p": 2}],
            "model": {"x": 1, "y": 2}}"#;
        let right = r#"{"contract": "menura.migration_report", "changes": [{"p": 2}, {"p": 1}],
            "model": {"y": 2, "x": 1}}"#;
        assert!(equal_up_to_input_order(left, right));
        assert!(!equal_up_to_input_order(
            left,
            &right.replace("\"x\": 1", "\"x\": 3")
        ));
        let diff =
            |changes: &str| format!(r#"{{"contract": "menura.model_diff", "changes": {changes}}}"#);
        assert!(!equal_up_to_input_order(&diff("[1, 2]"), &diff("[2, 1]")));
    }

    fn plan(warnings: &str, sql: &str) -> String {
        format!(
            "{{\n  \"contract\": \"menura.compiled_plan\",\n  \"sql\": \"{sql}\",\n  \
             \"cache_key\": {{\n    \"key\": \"k\",\n    \"plan_hash\": \"h\"\n  }},\n  \
             \"warnings\": {warnings}\n}}"
        )
    }

    #[test]
    fn pln_2_4_a_plan_is_compared_byte_for_byte() {
        let warnings = "[\n    {\n      \"code\": \"W0E-01\",\n      \"location\": {\n        \
                        \"line\": 2\n      }\n    },\n    {\n      \"code\": \"W0E-02\"\n    }\n  ]";
        let moved = "[\n    {\n      \"code\": \"W0E-02\"\n    },\n    {\n      \"code\": \
                     \"W0E-01\",\n      \"location\": {\n        \"line\": 7\n      }\n    }\n  ]";
        let base = plan(warnings, "SELECT 1");
        assert!(equal_up_to_input_order(&base, &plan(moved, "SELECT 1")));
        assert!(!equal_up_to_input_order(
            &base,
            &plan(warnings, "SELECT  1")
        ));
        let reordered = base.replace(
            "\"key\": \"k\",\n    \"plan_hash\": \"h\"",
            "\"plan_hash\": \"h\",\n    \"key\": \"k\"",
        );
        assert!(!equal_up_to_input_order(&base, &reordered));
        let spaced = base.replace("\"sql\": ", "\"sql\":  ");
        assert!(!equal_up_to_input_order(&base, &spaced));
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

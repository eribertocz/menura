//! The `temporal` extension of a relationship, in E0.c: the shape of every declaration
//! (VAL-2.1, `E0C-02`, `E0C-03`).
//!
//! `temporal` is valid on relationships only (`E0C-05`), which the common rules check
//! (E0C-2.3). The names a declaration holds are resolved in E0.d (`E0C-67` to `E0C-69`) and
//! typed in E0.f (`E0C-70`, `E0C-71`), each against the [`TemporalDeclaration`] read here.

mod model;

pub(crate) use model::TemporalDeclaration;

use serde_json::{Map, Value as Json};

use super::diagnostic::{ExtensionCode, ExtensionDiagnostic};
use super::output::OwnEntry;
use super::problems::{Problems, report};

/// The extension key of a temporal relationship (VAL-2.1).
pub(crate) const TEMPORAL: &str = "temporal";

/// The key of the field of `from` that holds the time of a row (VAL-2.2).
const FROM_TIME: &str = "from_time";

/// The key of the `validity` population of `to` (VAL-2.3).
const POPULATION: &str = "population";

/// The temporal declarations of a model and the E0.c diagnostics about them.
#[derive(Debug, Default)]
pub(crate) struct TemporalDeclarations {
    /// Every declaration without problems, in document order.
    pub(crate) declarations: Vec<TemporalDeclaration>,
    /// At most one diagnostic per code and entry, in document order then by code.
    pub(crate) errors: Vec<ExtensionDiagnostic>,
}

/// Reads the `temporal` of every entry of `entries`, which are in document order.
pub(crate) fn validate_temporal(entries: &[OwnEntry]) -> TemporalDeclarations {
    let mut found = TemporalDeclarations::default();
    for entry in entries {
        let Some(value) = entry.extensions.get(TEMPORAL) else {
            continue;
        };
        let mut problems = Vec::new();
        let names = read_declaration(value, &mut problems);
        if let (Some((from_time, population)), true) = (names, problems.is_empty()) {
            found.declarations.push(TemporalDeclaration {
                relationship: entry.object_pointer.clone(),
                data: entry.data_pointer.clone(),
                location: entry.location,
                from_time,
                population,
            });
        }
        found.errors.extend(report(entry, problems));
    }
    found
}

/// The `from_time` and `population` of the declaration `value`, recording in `problems` a
/// value that is not an object, a missing key or a key that is not a non-empty string
/// (`E0C-03`), and an unknown key (`E0C-02`).
fn read_declaration(value: &Json, problems: &mut Problems) -> Option<(String, String)> {
    let Json::Object(members) = value else {
        problems.push((
            ExtensionCode::WrongValue,
            format!("`{TEMPORAL}` is {value}, not an object"),
        ));
        return None;
    };
    for key in members
        .keys()
        .filter(|key| *key != FROM_TIME && *key != POPULATION)
    {
        problems.push((
            ExtensionCode::UnknownKey,
            format!(
                "`{TEMPORAL}` has no key `{key}`; its keys are `{FROM_TIME}` and `{POPULATION}`"
            ),
        ));
    }
    let from_time = required_name(members, FROM_TIME, problems);
    let population = required_name(members, POPULATION, problems);
    Some((from_time?, population?))
}

/// The value of the key `key` of `members` when it is a non-empty string; `E0C-03` in
/// `problems` otherwise.
fn required_name(
    members: &Map<String, Json>,
    key: &str,
    problems: &mut Problems,
) -> Option<String> {
    match members.get(key) {
        Some(Json::String(name)) if !name.is_empty() => Some(name.clone()),
        Some(value) => {
            problems.push((
                ExtensionCode::WrongValue,
                format!("`{TEMPORAL}.{key}` is {value}, not a non-empty string"),
            ));
            None
        }
        None => {
            problems.push((
                ExtensionCode::WrongValue,
                format!("`{TEMPORAL}` has no `{key}`; it is required"),
            ));
            None
        }
    }
}

#[cfg(test)]
mod tests;

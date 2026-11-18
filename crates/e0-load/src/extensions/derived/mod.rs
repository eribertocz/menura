//! The `derived` extension, in E0.c: the shape of every declaration (DER-5.1.1, `E0C-02`,
//! `E0C-03`) and a derived metric with an `additivity` extension (`E0C-36`).
//!
//! `derived` is valid on metrics only (`E0C-05`), which the common rules check (E0C-2.3). The
//! inputs of a derived metric are resolved in E0.d (`E0C-32`, `E0C-33`, `E0C-37`), and its
//! formula and the dependency graph are checked in E0.f (`E0C-30`, `E0C-31`, `E0C-34`,
//! `E0C-35`), each against the [`DerivedDeclaration`] read here.

mod model;

pub(crate) use model::{DerivedDeclaration, DivisionByZero};

use serde_json::Value as Json;

use super::diagnostic::{ExtensionCode, ExtensionDiagnostic};
use super::output::OwnEntry;

/// The extension key of derived metrics (MDL-2).
const DERIVED: &str = "derived";

/// The extension key of additivity, which a derived metric may not have (DER-4.4, `E0C-36`).
const ADDITIVITY: &str = "additivity";

/// The only key of a `derived` declaration (DER-3).
const ON_DIVISION_BY_ZERO: &str = "on_division_by_zero";

/// The values of `on_division_by_zero` (DER-3).
const DIVISIONS_BY_ZERO: [(&str, DivisionByZero); 2] = [
    ("null", DivisionByZero::Null),
    ("error", DivisionByZero::Error),
];

/// The derived-metric declarations of a model and the E0.c diagnostics about them.
#[derive(Debug, Default)]
pub(crate) struct Derivations {
    /// Every declaration without problems, in document order.
    pub(crate) declarations: Vec<DerivedDeclaration>,
    /// At most one diagnostic per code and entry, in document order then by code.
    pub(crate) errors: Vec<ExtensionDiagnostic>,
}

/// Reads the `derived` of every entry of `entries`, which are in document order.
pub(crate) fn validate_derived(entries: &[OwnEntry]) -> Derivations {
    let mut found = Derivations::default();
    for entry in entries {
        let Some(value) = entry.extensions.get(DERIVED) else {
            continue;
        };
        let mut problems = Vec::new();
        let on_division_by_zero = read_declaration(value, &mut problems);
        if entry.extensions.contains_key(ADDITIVITY) {
            problems.push((
                ExtensionCode::DerivedAdditivity,
                "a derived metric has no additivity of its own; it follows its inputs (DER-4.4)"
                    .to_owned(),
            ));
        }
        if problems.is_empty() {
            found.declarations.push(DerivedDeclaration {
                metric: entry.object_pointer.clone(),
                data: entry.data_pointer.clone(),
                location: entry.location,
                on_division_by_zero,
            });
        }
        found.errors.extend(report(entry, problems));
    }
    found
}

/// The value of `on_division_by_zero` of the declaration `value`, recording in `problems` a
/// value that is not an object or a listed string (`E0C-03`) and an unknown key (`E0C-02`).
fn read_declaration(
    value: &Json,
    problems: &mut Vec<(ExtensionCode, String)>,
) -> Option<DivisionByZero> {
    let Json::Object(members) = value else {
        problems.push((
            ExtensionCode::WrongValue,
            format!("`derived` is {value}, not an object"),
        ));
        return None;
    };
    for key in members.keys().filter(|key| *key != ON_DIVISION_BY_ZERO) {
        problems.push((
            ExtensionCode::UnknownKey,
            format!("`derived` has no key `{key}`; its only key is `{ON_DIVISION_BY_ZERO}`"),
        ));
    }
    let value = members.get(ON_DIVISION_BY_ZERO)?;
    let found = DIVISIONS_BY_ZERO
        .iter()
        .find(|(name, _)| value.as_str() == Some(name))
        .map(|(_, division)| *division);
    if found.is_none() {
        problems.push((
            ExtensionCode::WrongValue,
            format!("`{ON_DIVISION_BY_ZERO}` is {value}, not the string `null` or `error`"),
        ));
    }
    found
}

/// One diagnostic per code of `problems`, in the order of the codes, at the `data` member of
/// `entry` (E0C-2.6).
fn report(
    entry: &OwnEntry,
    mut problems: Vec<(ExtensionCode, String)>,
) -> Vec<ExtensionDiagnostic> {
    problems.sort_by_key(|(code, _)| *code);
    let mut diagnostics: Vec<ExtensionDiagnostic> = Vec::new();
    for (code, message) in problems {
        match diagnostics.last_mut() {
            Some(last) if last.code == code => {
                last.message.push_str("; ");
                last.message.push_str(&message);
            }
            _ => diagnostics.push(ExtensionDiagnostic {
                code,
                pointer: entry.data_pointer.clone(),
                location: entry.location,
                message,
            }),
        }
    }
    diagnostics
}

#[cfg(test)]
pub(crate) mod fixture;
#[cfg(test)]
mod tests;

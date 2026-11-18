//! The `additivity` extension, in E0.c: the keys and values of every declaration (ADD-5.1.1 to
//! ADD-5.1.4) and `over` outside `entity` (`E0C-15`).
//!
//! The fields a declaration names are checked for reachability in E0.e (`E0C-16`), and the
//! kind of `over` and the population in E0.f (`E0C-12`, `E0C-17`), each against the
//! [`AdditivityDeclaration`] read here. E0.g checks it against the deduced additivity
//! (E0G-4.1, E0G-4.2).

mod declaration;
mod model;
mod reference;
mod values;

pub(crate) use model::{
    AdditivityDeclaration, DeclaredAdditivity, FieldReference, NullHandling, SemiAdditive,
};
pub(crate) use reference::field_reference;
pub(crate) use values::GRAINS;

use super::diagnostic::{ExtensionCode, ExtensionDiagnostic};
use super::output::OwnEntry;
use declaration::{Problems, read_declaration};

/// The extension key of additivity (MDL-2).
const ADDITIVITY: &str = "additivity";

/// The additivity declarations of a model and the E0.c diagnostics about them.
#[derive(Debug, Default)]
pub(crate) struct Additivities {
    /// Every declaration without problems, in document order.
    pub(crate) declarations: Vec<AdditivityDeclaration>,
    /// At most one diagnostic per code and entry, in document order then by code.
    pub(crate) errors: Vec<ExtensionDiagnostic>,
}

/// Reads the `additivity` of every entry of `entries`, which are in document order.
pub(crate) fn validate_additivity(entries: &[OwnEntry]) -> Additivities {
    let mut found = Additivities::default();
    for entry in entries {
        let Some(value) = entry.extensions.get(ADDITIVITY) else {
            continue;
        };
        let reading = read_declaration(value);
        found.errors.extend(report(entry, reading.problems));
        if let Some(additivity) = reading.additivity {
            found.declarations.push(AdditivityDeclaration {
                metric: entry.object_pointer.clone(),
                data: entry.data_pointer.clone(),
                location: entry.location,
                additivity,
            });
        }
    }
    found
}

/// One diagnostic per code of `problems`, in the order of the codes, at the `data` member of
/// `entry` (E0C-2.6).
fn report(entry: &OwnEntry, mut problems: Problems) -> Vec<ExtensionDiagnostic> {
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
                message: format!("{}: {message}", summary(code)),
            }),
        }
    }
    diagnostics
}

fn summary(code: ExtensionCode) -> &'static str {
    match code {
        ExtensionCode::UnknownKey => "unknown additivity key",
        ExtensionCode::NonAdditiveKey => "a non_additive declaration has another key",
        ExtensionCode::MissingAdditivityKey => "a required additivity key is missing",
        ExtensionCode::CarryForwardKeys => "the carry_forward keys do not match `missing`",
        ExtensionCode::SelectionPair => "`selection` does not match `aggregation`",
        ExtensionCode::OverInEntity => "`over` is part of `entity`",
        _ => "wrong additivity value",
    }
}

#[cfg(test)]
pub(crate) mod fixture;
#[cfg(test)]
mod tests;

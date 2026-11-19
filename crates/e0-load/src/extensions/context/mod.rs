//! The `context` extension, in E0.c: the shape of every declaration (CTX-5.1.1, CTX-5.1.2:
//! `E0C-02`, `E0C-03`, `E0C-62`) and a context metric with an `additivity` extension
//! (`E0C-36`).
//!
//! `context` is valid on model metrics only, not on a dataset metric of the development format
//! (`E0C-05`), which the common rules check (E0C-2.3). The base and the removed fields are
//! resolved in E0.d (`E0C-60`, `E0C-63`, `E0C-64`), the time dimension is checked in E0.e and
//! E0.f (`E0C-61`), and the metric graph in E0.f (`E0C-34`), each against the
//! [`ContextDeclaration`] read here.

mod declaration;
mod model;

pub(crate) use model::{ContextDeclaration, Modifier, Removed};

use super::diagnostic::{ExtensionCode, ExtensionDiagnostic};
use super::output::OwnEntry;
use super::problems::{Problems, report};
use declaration::read_context;

/// The extension key of context metrics (MDL-2).
pub(crate) const CONTEXT: &str = "context";

/// The extension key of additivity, which a context metric may not have (CTX-5.1.1, DER-4.4).
const ADDITIVITY: &str = "additivity";

/// The context declarations of a model and the E0.c diagnostics about them.
#[derive(Debug, Default)]
pub(crate) struct Contexts {
    /// Every declaration without problems, in document order.
    pub(crate) declarations: Vec<ContextDeclaration>,
    /// At most one diagnostic per code and entry, in document order then by code.
    pub(crate) errors: Vec<ExtensionDiagnostic>,
}

/// Reads the `context` of every entry of `entries`, which are in document order.
pub(crate) fn validate_context(entries: &[OwnEntry]) -> Contexts {
    let mut found = Contexts::default();
    for entry in entries {
        let Some(value) = entry.extensions.get(CONTEXT) else {
            continue;
        };
        let mut problems = Problems::new();
        let declaration = read_context(value, &mut problems);
        if entry.extensions.contains_key(ADDITIVITY) {
            problems.push((
                ExtensionCode::DerivedAdditivity,
                "a context metric has no additivity of its own; it is non-additive (CTX-5.1.7)"
                    .to_owned(),
            ));
        }
        if let (Some((base, modifier)), true) = (declaration, problems.is_empty()) {
            found.declarations.push(ContextDeclaration {
                metric: entry.object_pointer.clone(),
                data: entry.data_pointer.clone(),
                location: entry.location,
                base,
                modifier,
            });
        }
        found.errors.extend(report(entry, problems));
    }
    found
}

#[cfg(test)]
pub(crate) mod fixture;
#[cfg(test)]
mod tests;

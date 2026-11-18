//! Stage E0.c: the common extension rules (E0C-1, E0C-2, MDL-2).
//!
//! Finds the extension entries of the selected semantic model that are ours, `custom_extensions`
//! in the release format and `nonportable_extensions` in the development format (MDL-2.4), checks
//! their version and the placement of their keys, and hands the extension keys of each object
//! to the validation of each extension (E0C-2.5): the shape and keys of populations are checked
//! here (POP-4.1 to POP-4.3), those of additivity declarations (ADD-5.1.1 to ADD-5.1.4) and
//! those of derived-metric declarations (DER-5.1.1).
//! Other vendors' entries, and `COMMON` entries
//! of another namespace, are kept in the data tree and never interpreted (E0C-1.1, MDL-2.5).

pub(crate) mod additivity;
pub(crate) mod derived;
mod diagnostic;
mod entry;
mod envelope;
mod keys;
mod members;
mod object_data;
mod objects;
mod output;
pub(crate) mod populations;

pub(crate) use diagnostic::{ExtensionCode, ExtensionDiagnostic};
use entry::{EntryKind, classify};
use envelope::extension_keys;
use objects::{EntryNode, extension_entries};
pub(crate) use output::{Extensions, ObjectKind};
use output::{ExtensionsFailure, OwnEntry};
pub(crate) use populations::{Population, per_entry};

use crate::pointer::{JsonPointer, position};
use crate::read::Node;
use crate::structure::SelectedModel;

/// The namespace of the compiler's own entries when the load option `namespace` is not given
/// (MDL-2.3).
pub(crate) const DEFAULT_NAMESPACE: &str = "semantic_compiler";

/// Applies the common extension rules to the selected semantic model of `document`.
///
/// `namespace` is the load option of the same name (MDL-2.3); [`DEFAULT_NAMESPACE`] when not
/// given. Entries and diagnostics are in document order, then by code (E0-4.3).
///
/// # Errors
///
/// Returns an [`ExtensionsFailure`] with every rejecting diagnostic, `E0C-01` to `E0C-06`
/// (`E0C-03` also for a development-line `data` without a JSON form, E0C-2.8),
/// the E0.c codes of populations (`E0C-02`, `E0C-03`, `E0C-20`, `E0C-21`) and of additivity
/// (`E0C-02`, `E0C-03`, `E0C-10`, `E0C-11`, `E0C-13` to `E0C-15`) and of derived metrics
/// (`E0C-02`, `E0C-03`, `E0C-36`), and the `W0C-01` warnings.
pub fn validate_extensions(
    document: &Node,
    model: &SelectedModel<'_>,
    namespace: &str,
) -> Result<Extensions, ExtensionsFailure> {
    let mut entries = Vec::new();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut previous_owner: Option<JsonPointer> = None;
    for entry in extension_entries(model.node, &model.pointer) {
        let data = match classify(&entry, namespace) {
            EntryKind::Foreign => continue,
            EntryKind::NotJson(warning) => {
                warnings.push(warning);
                continue;
            }
            EntryKind::Ours(data) => data,
        };
        if previous_owner.as_ref() == Some(&entry.object_pointer) {
            errors.push(several_entries(&entry));
        }
        previous_owner = Some(entry.object_pointer.clone());
        let (data_pointer, location) = (data.pointer.clone(), data.node.location);
        match extension_keys(data, entry.object) {
            Ok(extensions) => entries.push(OwnEntry {
                object: entry.object,
                object_pointer: entry.object_pointer,
                data_pointer,
                location,
                extensions,
            }),
            Err(found) => errors.extend(found),
        }
    }
    entries.sort_by_cached_key(|entry| position(document, &entry.data_pointer));
    let populations = populations::validate_populations(&entries);
    errors.extend(populations.errors);
    let additivity = additivity::validate_additivity(&entries);
    errors.extend(additivity.errors);
    let derived = derived::validate_derived(&entries);
    errors.extend(derived.errors);
    let warnings = sorted(document, warnings);
    if !errors.is_empty() {
        return Err(ExtensionsFailure {
            errors: sorted(document, errors),
            warnings,
        });
    }
    Ok(Extensions {
        entries,
        populations: populations.populations,
        additivity: additivity.declarations,
        derived: derived.declarations,
        warnings,
    })
}

/// The `E0C-04` diagnostic of an entry of ours that follows another on the same object
/// (E0C-2.2, MDL-2.1).
fn several_entries(entry: &EntryNode<'_>) -> ExtensionDiagnostic {
    ExtensionDiagnostic {
        code: ExtensionCode::SeveralEntries,
        pointer: entry.pointer.clone(),
        location: entry.node.location,
        message: format!(
            "the {} already has an entry of the compiler; an object has at most one",
            entry.object.as_str()
        ),
    }
}

/// Sorts diagnostics in document order, then by code (E0-4.3).
fn sorted(document: &Node, mut diagnostics: Vec<ExtensionDiagnostic>) -> Vec<ExtensionDiagnostic> {
    diagnostics.sort_by_cached_key(|diagnostic| {
        (position(document, &diagnostic.pointer), diagnostic.code)
    });
    diagnostics
}

#[cfg(test)]
mod development_tests;
#[cfg(test)]
mod tests;

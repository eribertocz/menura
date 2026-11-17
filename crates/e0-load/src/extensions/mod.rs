//! Stage E0.c: the common extension rules (E0C-1, E0C-2, MDL-2).
//!
//! Finds the `custom_extensions` entries of the selected semantic model that are ours, checks
//! their version and the placement of their keys, and hands the extension keys of each object
//! to the validation of each extension (E0C-2.5). Other vendors' entries, and `COMMON` entries
//! of another namespace, are kept in the data tree and never interpreted (E0C-1.1, MDL-2.5).

mod diagnostic;
mod entry;
mod envelope;
mod keys;
mod members;
mod objects;
mod output;

use diagnostic::{ExtensionCode, ExtensionDiagnostic};
use entry::{EntryKind, classify};
use envelope::extension_keys;
use objects::{EntryNode, extension_entries};
pub(crate) use output::{Extensions, ObjectKind};
use output::{ExtensionsFailure, OwnEntry};

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
/// Returns an [`ExtensionsFailure`] with every rejecting diagnostic, `E0C-01` to `E0C-05`, and
/// the `W0C-01` warnings.
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
    let warnings = sorted(document, warnings);
    if !errors.is_empty() {
        return Err(ExtensionsFailure {
            errors: sorted(document, errors),
            warnings,
        });
    }
    entries.sort_by_cached_key(|entry| position(document, &entry.data_pointer));
    Ok(Extensions { entries, warnings })
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
mod tests;

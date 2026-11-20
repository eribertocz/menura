//! Stage E0.c: the common extension rules (E0C-1, E0C-2, MDL-2).
//!
//! Finds the extension entries of the selected semantic model that are ours, `custom_extensions`
//! in the release format and `nonportable_extensions` in the development format (MDL-2.4), checks
//! their version and the placement of their keys, and hands the extension keys of each object
//! to the validation of each extension (E0C-2.5): the shape and keys of populations are checked
//! here (POP-4.1 to POP-4.3), those of additivity declarations (ADD-5.1.1 to ADD-5.1.4) and
//! those of derived-metric declarations (DER-5.1.1), those of context declarations
//! (CTX-5.1.1, CTX-5.1.2), those of the security extension (SCY-5.1.2, SCY-5.1.3,
//! SCY-5.1.5) and those of summaries (SUM-6.1.1, SUM-6.1.6). Which datasets are summaries,
//! whatever E0.c reports, comes from [`summaries::datasets::summary_datasets`] (SUM-6.1.7).
//! The access every dataset is read with, and the warnings `W0C-40` and `W0C-41`, come from
//! [`security::resolution::resolve_access`] (SCY-5.1.7), which needs the load option
//! `grant_authenticated_to_all`.
//! Other vendors' entries, and `COMMON` entries
//! of another namespace, are kept in the data tree and never interpreted (E0C-1.1, MDL-2.5).

pub(crate) mod additivity;
pub(crate) mod context;
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
mod problems;
pub(crate) mod security;
pub(crate) mod summaries;

pub(crate) use diagnostic::{ExtensionCode, ExtensionDiagnostic};
use entry::{EntryKind, classify};
use envelope::extension_keys;
use objects::{EntryNode, extension_entries};
use output::ExtensionsFailure;
pub(crate) use output::OwnEntry;
pub(crate) use output::{Extensions, ObjectKind};
pub(crate) use populations::{Population, per_entry};
pub(crate) use problems::{Problems, report_at};

use crate::pointer::{JsonPointer, position};
use crate::read::Node;
use crate::structure::SelectedModel;

/// The namespace of the compiler's own entries when the load option `namespace` is not given
/// (MDL-2.3).
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the stage function takes the namespace from the load options; only the substage tests use the default"
    )
)]
pub(crate) const DEFAULT_NAMESPACE: &str = "menura";

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
/// (`E0C-02`, `E0C-03`, `E0C-10`, `E0C-11`, `E0C-13` to `E0C-15`), of derived metrics
/// (`E0C-02`, `E0C-03`, `E0C-36`) and of context metrics (`E0C-02`, `E0C-03`, `E0C-36`,
/// `E0C-62`), of the security extension (`E0C-02`, `E0C-03`, `E0C-41` to `E0C-45`,
/// `E0C-47`), of summaries (`E0C-02`, `E0C-03`, `E0C-50`, `E0C-55`, `E0C-56`), and the
/// `W0C-01` warnings.
pub fn validate_extensions(
    document: &Node,
    model: &SelectedModel<'_>,
    namespace: &str,
) -> Result<Extensions, ExtensionsFailure> {
    let EntriesOfOurs {
        mut entries,
        mut errors,
        warnings,
    } = entries_of_ours(model, namespace);
    entries.sort_by_cached_key(|entry| position(document, &entry.data_pointer));
    let populations = populations::validate_populations(&entries);
    errors.extend(populations.errors);
    let additivity = additivity::validate_additivity(&entries);
    errors.extend(additivity.errors);
    let derived = derived::validate_derived(&entries);
    errors.extend(derived.errors);
    let context = context::validate_context(&entries);
    errors.extend(context.errors);
    let security = security::validate_security(&model.pointer, &entries);
    errors.extend(security.errors);
    let summaries = summaries::validate_summaries(model, &entries);
    errors.extend(summaries.errors);
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
        context: context.declarations,
        security: security.security,
        summaries: summaries.declarations,
        warnings,
    })
}

/// The entries of ours of a model, before the validation of each extension, with the
/// diagnostics of the common rules found while reading them.
struct EntriesOfOurs {
    /// The entries whose keys hold, in the order they were found.
    entries: Vec<OwnEntry>,
    /// The rejecting diagnostics of the common rules, `E0C-01` to `E0C-06`.
    errors: Vec<ExtensionDiagnostic>,
    /// The `W0C-01` warnings.
    warnings: Vec<ExtensionDiagnostic>,
}

/// Reads the entries of ours of the selected semantic model `model`, with the load option
/// `namespace` (MDL-2.3), and applies the common rules to them (E0C-1, E0C-2).
fn entries_of_ours(model: &SelectedModel<'_>, namespace: &str) -> EntriesOfOurs {
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
    EntriesOfOurs {
        entries,
        errors,
        warnings,
    }
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

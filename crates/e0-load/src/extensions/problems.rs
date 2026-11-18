//! The diagnostics of one entry of ours, from the problems an extension found in it.

use super::diagnostic::{ExtensionCode, ExtensionDiagnostic};
use super::output::OwnEntry;
use crate::pointer::JsonPointer;
use crate::read::Location;

/// The problems an extension found in one entry: each code with a description in English.
pub(crate) type Problems = Vec<(ExtensionCode, String)>;

/// One diagnostic per code of `problems`, in the order of the codes, at the `data` member of
/// `entry` (E0C-2.6); the descriptions of one code are joined.
pub(super) fn report(entry: &OwnEntry, problems: Problems) -> Vec<ExtensionDiagnostic> {
    report_at(&entry.data_pointer, entry.location, problems)
}

/// One diagnostic per code of `problems`, in the order of the codes, at the node `pointer`
/// that starts at `location`; the descriptions of one code are joined.
pub(crate) fn report_at(
    pointer: &JsonPointer,
    location: Location,
    mut problems: Problems,
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
                pointer: pointer.clone(),
                location,
                message,
            }),
        }
    }
    diagnostics
}

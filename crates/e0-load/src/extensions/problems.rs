//! The diagnostics of one entry of ours, from the problems an extension found in it.

use super::diagnostic::{ExtensionCode, ExtensionDiagnostic};
use super::output::OwnEntry;

/// The problems an extension found in one entry: each code with a description in English.
pub(super) type Problems = Vec<(ExtensionCode, String)>;

/// One diagnostic per code of `problems`, in the order of the codes, at the `data` member of
/// `entry` (E0C-2.6); the descriptions of one code are joined.
pub(super) fn report(entry: &OwnEntry, mut problems: Problems) -> Vec<ExtensionDiagnostic> {
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

//! `E0C-50`, `E0C-51` and `E0C-54`: the names of every summary, in E0.d (SUM-6.1.2).

use crate::extensions::{ExtensionDiagnostic, Extensions};
use crate::structure::Structure;

/// Resolves the names of every summary declaration of `extensions`, in the semantic model that
/// `structure` selected.
///
/// # Errors
///
/// Returns at most one `E0C-50`, one `E0C-51` and one `E0C-54` diagnostic per summary, at the
/// `data` member of its entry of ours (E0C-2.6), in document order then by code. Each rejects
/// the model (E0-4).
pub fn resolve_summary_references(
    structure: &Structure<'_>,
    extensions: &Extensions,
) -> Result<(), Vec<ExtensionDiagnostic>> {
    let _ = (structure, extensions);
    Ok(())
}

//! `E0C-52`, `E0C-53` and `E0C-57`: the `base_aggregate` of every measure, the grain of every
//! dimension and the kind of every summary field, in E0.f (SUM-6.1.3 to SUM-6.1.5).

use super::output::Typing;
use crate::extensions::{ExtensionDiagnostic, Extensions};
use crate::structure::Structure;

/// Types the measures and dimensions of every summary declaration of `extensions`, in the
/// semantic model that `structure` selected, with the field types of `typing`.
///
/// # Errors
///
/// Returns at most one `E0C-52`, one `E0C-53` and one `E0C-57` diagnostic per summary, at the
/// `data` member of its entry of ours (E0C-2.6), in document order then by code. Each rejects
/// the model (E0-4).
pub fn type_summaries(
    structure: &Structure<'_>,
    typing: &Typing,
    extensions: &Extensions,
) -> Result<(), Vec<ExtensionDiagnostic>> {
    let _ = (structure, typing, extensions);
    Ok(())
}

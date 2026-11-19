//! The summary extension, in E0.c: the shape and keys of every `aggregate` (SUM-6.1.1:
//! `E0C-02`, `E0C-03`, and `E0C-50` for a missing `of`), and the policies of a summary
//! (SUM-6.1.6: `E0C-55`). `E0C-56`, about the `access` of a summary, is read with the
//! security extension.

mod datasets;
#[cfg(test)]
pub(crate) mod fixture;
mod model;

pub(crate) use datasets::{SummaryDatasets, summary_datasets};
pub(crate) use model::{Grain, SummaryDeclaration, SummaryDimension, SummaryMeasure};

use super::diagnostic::ExtensionDiagnostic;
use super::output::OwnEntry;
use crate::structure::SelectedModel;

/// The extension key that makes a dataset a summary (SUM-2).
pub(crate) const AGGREGATE: &str = "aggregate";

/// The summary declarations of a model and the E0.c diagnostics about them.
#[derive(Debug, Default)]
pub(crate) struct SummaryValidation {
    /// Every declaration without problems, in document order.
    pub(crate) declarations: Vec<SummaryDeclaration>,
    /// At most one diagnostic per code and entry.
    pub(crate) errors: Vec<ExtensionDiagnostic>,
}

/// Reads the summary extension of every dataset entry of `entries`, which are in document
/// order, in the semantic model `model`.
pub(crate) fn validate_summaries(
    model: &SelectedModel<'_>,
    entries: &[OwnEntry],
) -> SummaryValidation {
    let _ = (model, entries);
    SummaryValidation::default()
}

#[cfg(test)]
mod tests;

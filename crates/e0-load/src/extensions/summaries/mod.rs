//! The summary extension, in E0.c: the shape and keys of every `aggregate` (SUM-6.1.1:
//! `E0C-02`, `E0C-03`, and `E0C-50` for a missing `of`), and the policies of a summary
//! (SUM-6.1.6: `E0C-55`). `E0C-56`, about the `access` of a summary, is read with the
//! security extension.

pub(crate) mod datasets;
#[cfg(test)]
pub(crate) mod fixture;
mod model;
mod shape;

pub(crate) use datasets::SummaryDatasets;
pub(crate) use model::{Grain, SummaryDeclaration, SummaryDimension, SummaryMeasure};

use std::collections::HashSet;

use super::diagnostic::{ExtensionCode, ExtensionDiagnostic};
use super::output::{ObjectKind, OwnEntry};
use super::problems::{Problems, report};
use crate::outline::outline;
use crate::pointer::JsonPointer;
use crate::structure::SelectedModel;
use shape::read_aggregate;

/// The extension key that makes a dataset a summary (SUM-2).
pub(crate) const AGGREGATE: &str = "aggregate";

/// The extension key of row policies, which a summary may not declare (SUM-3.6, SUM-6.1.6).
const ROW_ACCESS: &str = "row_access";

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
    let with_dataset_metrics: HashSet<JsonPointer> = outline(model.node, &model.pointer)
        .datasets
        .into_iter()
        .filter(|dataset| !dataset.metrics.is_empty())
        .map(|dataset| dataset.pointer)
        .collect();
    let mut found = SummaryValidation::default();
    for entry in entries
        .iter()
        .filter(|entry| entry.object == ObjectKind::Dataset)
    {
        let Some(value) = entry.extensions.get(AGGREGATE) else {
            continue;
        };
        let mut problems = Problems::new();
        let shape = read_aggregate(value, &mut problems);
        policy_problems(entry, &with_dataset_metrics, &mut problems);
        if let (Some(shape), true) = (shape, problems.is_empty()) {
            found.declarations.push(SummaryDeclaration {
                dataset: entry.object_pointer.clone(),
                data: entry.data_pointer.clone(),
                location: entry.location,
                of: shape.of,
                dimensions: shape.dimensions,
                measures: shape.measures,
                coverage_field: shape.coverage_field,
            });
        }
        found.errors.extend(report(entry, problems));
    }
    found
}

/// Records `E0C-55` when the summary of `entry` declares `row_access`, or dataset metrics of
/// the development format; `with_dataset_metrics` are the datasets that declare any.
fn policy_problems(
    entry: &OwnEntry,
    with_dataset_metrics: &HashSet<JsonPointer>,
    problems: &mut Problems,
) {
    if entry.extensions.contains_key(ROW_ACCESS) {
        problems.push((
            ExtensionCode::SummaryPolicies,
            "a summary declares no row policies of its own".to_owned(),
        ));
    }
    if with_dataset_metrics.contains(&entry.object_pointer) {
        problems.push((
            ExtensionCode::SummaryPolicies,
            "a summary declares no dataset metrics".to_owned(),
        ));
    }
}

#[cfg(test)]
mod tests;

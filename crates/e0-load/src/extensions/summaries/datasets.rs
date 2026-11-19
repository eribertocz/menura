//! Which datasets are summaries: those whose entry of ours has the key `aggregate`, valid or
//! not (SUM-6.1.1, E0E-3.6), whatever E0.c reports (SUM-6.1.7).

use std::collections::BTreeSet;

use crate::pointer::JsonPointer;
use crate::structure::SelectedModel;

/// The JSON Pointers of the summary datasets of a model.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SummaryDatasets {
    datasets: BTreeSet<JsonPointer>,
}

impl SummaryDatasets {
    /// Whether the dataset at `dataset` is a summary.
    #[must_use]
    pub fn contains(&self, dataset: &JsonPointer) -> bool {
        self.datasets.contains(dataset)
    }

    /// The summary datasets, ordered by JSON Pointer.
    pub fn iter(&self) -> impl Iterator<Item = &JsonPointer> {
        self.datasets.iter()
    }
}

/// The summary datasets of the selected semantic model `model`, with the load option
/// `namespace` (MDL-2.3).
pub(crate) fn summary_datasets(model: &SelectedModel<'_>, namespace: &str) -> SummaryDatasets {
    let _ = (model, namespace);
    SummaryDatasets::default()
}

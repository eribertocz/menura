//! Which datasets are summaries: those whose entry of ours has the key `aggregate`, valid or
//! not (SUM-6.1.1, E0E-3.6), whatever E0.c reports (SUM-6.1.7).

use std::collections::BTreeSet;

use super::AGGREGATE;
use crate::extensions::entry::{EntryKind, classify};
use crate::extensions::objects::extension_entries;
use crate::extensions::output::ObjectKind;
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
/// `namespace` (MDL-2.3). The entries are read as E0.c reads them (E0C-1), without applying
/// any other rule of E0.c, so a summary is recognized even when E0.c rejects its entry.
pub(crate) fn summary_datasets(model: &SelectedModel<'_>, namespace: &str) -> SummaryDatasets {
    let datasets = extension_entries(model.node, &model.pointer)
        .into_iter()
        .filter(|entry| entry.object == ObjectKind::Dataset)
        .filter(|entry| {
            matches!(
                classify(entry, namespace),
                EntryKind::Ours(data) if data.members.contains_key(AGGREGATE)
            )
        })
        .map(|entry| entry.object_pointer)
        .collect();
    SummaryDatasets { datasets }
}

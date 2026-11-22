//! The paths a description reports: over the authorized subgraph (E2-4.8) restricted to the
//! listed datasets (DSC-3.2a, DSC-3.7), through many-to-one steps (E0E-2.3) that never repeat
//! a dataset (E0E-2.4).

use std::collections::BTreeSet;

use menura_contracts::types::audit_record::Access;
use menura_contracts::types::descriptions::AmbiguousPath;
use menura_model::internal_model::{FieldPath, InternalModel, NormalizedName, Relationship};
use menura_security::Authorization;

use super::listing::Listing;
use super::steps::{Path, Steps};

/// The graph of the steps a description may report.
pub(super) struct DescribedGraph<'model> {
    /// The listed datasets, ordered by name.
    datasets: Vec<&'model NormalizedName>,
    /// The authorized steps between them.
    steps: Steps<'model>,
}

impl<'model> DescribedGraph<'model> {
    /// The edges of the relationship graph of `model` whose two ends are listed and whose
    /// keys the user of `authorization` may read (E2-4.2, E2-4.8).
    pub(super) fn new(
        model: &'model InternalModel,
        listing: &Listing<'model>,
        authorization: &Authorization<'_>,
    ) -> Self {
        let edges = model.parts().graph.edges.iter().filter(|edge| {
            model.relationship(&edge.relationship).is_some_and(|found| {
                listing.lists_dataset(&edge.from)
                    && listing.lists_dataset(&edge.to)
                    && keys_readable(model, found, authorization)
            })
        });
        let datasets = listing
            .datasets
            .iter()
            .map(|listed| &listed.dataset.name)
            .collect();
        Self {
            datasets,
            steps: Steps::new(edges.map(|edge| (&edge.relationship, &edge.from, &edge.to))),
        }
    }

    /// The listed datasets every dataset of `homes` reaches, ordered by name (E3-3.4). A
    /// dataset reaches itself.
    pub(super) fn reached_by_all(&self, homes: &[NormalizedName]) -> Vec<String> {
        let reached: Vec<BTreeSet<&NormalizedName>> = homes
            .iter()
            .map(|home| self.steps.reached_from(home))
            .collect();
        self.datasets
            .iter()
            .filter(|dataset| reached.iter().all(|found| found.contains(*dataset)))
            .map(|dataset| dataset.as_str().to_owned())
            .collect()
    }

    /// Every ordered pair of listed datasets with at least two paths, by `from` then `to`,
    /// each with its two smallest paths (DSC-3.9, DSC-3.10).
    pub(super) fn ambiguous_paths(&self) -> Vec<AmbiguousPath> {
        let mut pairs = Vec::new();
        for from in &self.datasets {
            for to in self.steps.reached_from(from) {
                if to == *from {
                    continue;
                }
                let found = self.steps.two_smallest(from, to);
                if found.len() == 2 {
                    pairs.push(AmbiguousPath {
                        from: from.as_str().to_owned(),
                        to: to.as_str().to_owned(),
                        paths: found.iter().map(names).collect(),
                    });
                }
            }
        }
        pairs
    }
}

/// The relationship names of `path`, as the description writes them (DSC-3.10).
fn names(path: &Path<'_>) -> Vec<String> {
    path.iter()
        .map(|(relationship, _)| relationship.as_str().to_owned())
        .collect()
}

/// Whether the user of `authorization` may read every key column of `relationship` (E2-4.2):
/// a field by its field restrictions, any other column by the restrictions that protect that
/// source column (SCY-3.1, SCY-3.1a).
fn keys_readable(
    model: &InternalModel,
    relationship: &Relationship,
    authorization: &Authorization<'_>,
) -> bool {
    let readable = |dataset: &NormalizedName, column: &NormalizedName| {
        let is_field = model
            .dataset(dataset)
            .is_some_and(|found| found.field(column).is_some());
        if is_field {
            let path = FieldPath {
                dataset: dataset.clone(),
                field: column.clone(),
            };
            authorization.field(&path).access == Access::Granted
        } else {
            authorization.key_column(dataset, column)
        }
    };
    relationship
        .from_columns
        .iter()
        .all(|column| readable(&relationship.from, column))
        && relationship
            .to_columns
            .iter()
            .all(|column| readable(&relationship.to, column))
}

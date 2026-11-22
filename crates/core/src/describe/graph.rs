//! The paths a description reports: over the authorized subgraph (E2-4.8) restricted to the
//! listed datasets (DSC-3.2a, DSC-3.7), through many-to-one steps (E0E-2.3) that never repeat
//! a dataset (E0E-2.4).

use std::collections::{BTreeMap, BTreeSet};

use menura_contracts::types::audit_record::Access;
use menura_contracts::types::descriptions::AmbiguousPath;
use menura_model::internal_model::{FieldPath, InternalModel, NormalizedName, Relationship};
use menura_security::Authorization;

use super::listing::Listing;

/// One many-to-one step: the relationship and the dataset it reaches.
type Step<'model> = (&'model NormalizedName, &'model NormalizedName);

/// A path: its relationships, in order.
type Path<'model> = Vec<&'model NormalizedName>;

/// The graph of the steps a description may report.
pub(super) struct DescribedGraph<'model> {
    /// The listed datasets, ordered by name.
    datasets: Vec<&'model NormalizedName>,
    /// For each dataset, the steps from it, ordered by relationship name.
    steps: BTreeMap<&'model NormalizedName, Vec<Step<'model>>>,
}

impl<'model> DescribedGraph<'model> {
    /// The edges of the relationship graph of `model` whose two ends are listed and whose
    /// keys the user of `authorization` may read (E2-4.2, E2-4.8).
    pub(super) fn new(
        model: &'model InternalModel,
        listing: &Listing<'model>,
        authorization: &Authorization<'_>,
    ) -> Self {
        let mut steps: BTreeMap<&NormalizedName, Vec<Step<'model>>> = BTreeMap::new();
        for edge in &model.parts().graph.edges {
            let authorized = model.relationship(&edge.relationship).is_some_and(|found| {
                listing.lists_dataset(&edge.from)
                    && listing.lists_dataset(&edge.to)
                    && keys_readable(model, found, authorization)
            });
            if authorized {
                steps
                    .entry(&edge.from)
                    .or_default()
                    .push((&edge.relationship, &edge.to));
            }
        }
        let datasets = listing
            .datasets
            .iter()
            .map(|listed| &listed.dataset.name)
            .collect();
        Self { datasets, steps }
    }

    /// The listed datasets every dataset of `homes` reaches, ordered by name (E3-3.4). A
    /// dataset reaches itself.
    pub(super) fn reached_by_all(&self, homes: &[NormalizedName]) -> Vec<String> {
        let reached: Vec<BTreeSet<&NormalizedName>> =
            homes.iter().map(|home| self.reached_from(home)).collect();
        self.datasets
            .iter()
            .filter(|dataset| reached.iter().all(|found| found.contains(*dataset)))
            .map(|dataset| dataset.as_str().to_owned())
            .collect()
    }

    /// The datasets `from` reaches, itself included.
    fn reached_from<'name>(
        &'name self,
        from: &'name NormalizedName,
    ) -> BTreeSet<&'name NormalizedName> {
        let mut reached = BTreeSet::from([from]);
        let mut pending = vec![from];
        while let Some(dataset) = pending.pop() {
            for (_, to) in self.steps.get(dataset).into_iter().flatten() {
                if reached.insert(*to) {
                    pending.push(*to);
                }
            }
        }
        reached
    }

    /// Every ordered pair of listed datasets with at least two paths, by `from` then `to`,
    /// each with its two smallest paths (DSC-3.9, DSC-3.10).
    pub(super) fn ambiguous_paths(&self) -> Vec<AmbiguousPath> {
        let mut pairs = Vec::new();
        for from in &self.datasets {
            for (to, found) in self.paths_from(from) {
                if found.count >= 2 {
                    pairs.push(AmbiguousPath {
                        from: from.as_str().to_owned(),
                        to: to.as_str().to_owned(),
                        paths: found.smallest.iter().map(|path| names(path)).collect(),
                    });
                }
            }
        }
        pairs
    }

    /// For every dataset reached from `from` by a non-empty path, how many paths lead there
    /// and the two smallest of them, by reached dataset.
    fn paths_from(
        &self,
        from: &'model NormalizedName,
    ) -> BTreeMap<&'model NormalizedName, Found<'model>> {
        let mut found: BTreeMap<&NormalizedName, Found<'model>> = BTreeMap::new();
        let mut pending: Vec<(Vec<&NormalizedName>, Path<'model>)> = vec![(vec![from], Vec::new())];
        while let Some((visited, path)) = pending.pop() {
            let Some(current) = visited.last() else {
                continue;
            };
            for (relationship, to) in self.steps.get(current).into_iter().flatten() {
                if visited.contains(to) {
                    continue;
                }
                let mut longer = path.clone();
                longer.push(*relationship);
                found.entry(*to).or_default().add(&longer);
                let mut further = visited.clone();
                further.push(*to);
                pending.push((further, longer));
            }
        }
        found
    }
}

/// The paths found to one dataset.
#[derive(Default)]
struct Found<'model> {
    /// How many paths lead there.
    count: usize,
    /// The two smallest, ordered by length, then by relationship names (DSC-3.9).
    smallest: Vec<Path<'model>>,
}

impl<'model> Found<'model> {
    /// Counts `path` and keeps it when it is among the two smallest.
    fn add(&mut self, path: &Path<'model>) {
        self.count += 1;
        self.smallest.push(path.clone());
        self.smallest
            .sort_by(|left, right| left.len().cmp(&right.len()).then_with(|| left.cmp(right)));
        self.smallest.truncate(2);
    }
}

/// The relationship names of `path`, as the description writes them (DSC-3.10).
fn names(path: &Path<'_>) -> Vec<String> {
    path.iter().map(|name| name.as_str().to_owned()).collect()
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

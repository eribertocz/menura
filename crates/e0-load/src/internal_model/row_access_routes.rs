//! `E0H-01`: a dataset that, in the declared relationship graph, reaches a dataset with
//! `row_access`, and whose every such route goes through an unusable relationship, is
//! unusable, because its row policies could not be propagated (E0-4.2a, E0H-2.3, E4-6.5).

use std::collections::{BTreeSet, HashSet};

use super::ModelInput;
use super::index::ModelIndex;
use crate::outline::ModelOutline;
use crate::pointer::JsonPointer;

/// The query error code of a dataset whose row policies cannot be propagated (E0H-5).
pub(super) const UNREACHABLE_ROW_ACCESS: &str = "E0H-01";

/// One declared relationship between two declared datasets.
struct DeclaredEdge<'model> {
    relationship: &'model JsonPointer,
    from: &'model JsonPointer,
    to: &'model JsonPointer,
}

/// The declared relationship graph of `model`, before any relationship is made unusable.
pub(super) struct DeclaredGraph<'model> {
    datasets: Vec<&'model JsonPointer>,
    edges: Vec<DeclaredEdge<'model>>,
    row_access: HashSet<&'model JsonPointer>,
}

impl<'model> DeclaredGraph<'model> {
    /// The declared graph of `model`, with the datasets that have row policies in `input`.
    pub(super) fn new(input: &'model ModelInput<'_, '_>, model: &'model ModelOutline<'_>) -> Self {
        let index = ModelIndex::new(model);
        let dataset = |name: Option<&crate::outline::DeclaredName<'_>>| {
            name.and_then(|name| index.dataset(&name.name))
                .map(|dataset| &dataset.pointer)
        };
        let edges = model
            .relationships
            .iter()
            .filter_map(|relationship| {
                Some(DeclaredEdge {
                    relationship: &relationship.pointer,
                    from: dataset(relationship.from.as_ref())?,
                    to: dataset(relationship.to.as_ref())?,
                })
            })
            .collect();
        Self {
            datasets: model
                .datasets
                .iter()
                .map(|dataset| &dataset.pointer)
                .collect(),
            edges,
            row_access: input
                .extensions
                .security
                .policies
                .iter()
                .map(|policy| &policy.dataset)
                .collect(),
        }
    }

    /// The datasets that reach a dataset with `row_access` along the declared graph, but not
    /// along the relationships `is_usable` accepts, in document order.
    pub(super) fn cut_off(
        &self,
        is_usable: impl Fn(&JsonPointer) -> bool,
    ) -> Vec<&'model JsonPointer> {
        self.datasets
            .iter()
            .copied()
            .filter(|dataset| self.reaches_row_access(dataset, |_| true))
            .filter(|dataset| !self.reaches_row_access(dataset, |edge| is_usable(edge)))
            .collect()
    }

    /// Whether `start` reaches, along at least one relationship that `follows` accepts, a
    /// dataset other than itself that has `row_access`.
    fn reaches_row_access(
        &self,
        start: &JsonPointer,
        follows: impl Fn(&JsonPointer) -> bool,
    ) -> bool {
        let mut seen: BTreeSet<&JsonPointer> = BTreeSet::new();
        let mut pending = vec![start];
        while let Some(current) = pending.pop() {
            for edge in self.edges.iter().filter(|edge| edge.from == current) {
                if !follows(edge.relationship) || !seen.insert(edge.to) {
                    continue;
                }
                if edge.to != start && self.row_access.contains(edge.to) {
                    return true;
                }
                pending.push(edge.to);
            }
        }
        false
    }
}

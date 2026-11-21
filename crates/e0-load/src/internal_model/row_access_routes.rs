//! `E0H-01`: a dataset D is unusable when, in the declared relationship graph, some unusable
//! relationship X → Y has X equal to D or reachable from D, and Y equal to or reaching a
//! dataset with `row_access`, even if other routes stay usable: E4-6.5 follows every route, so
//! losing one would drop a row policy (E0-4.2a, E0H-2.3). It is computed by reachability: one
//! backward pass from the datasets with `row_access`, then one backward pass from the sources
//! of those unusable relationships, taken by normalized name.

use std::collections::{HashMap, VecDeque};

use menura_model::internal_model::{NormalizedName, RowAccessCut};

use super::ModelInput;
use super::index::ModelIndex;
use super::located::declared;
use crate::outline::{DeclaredName, ModelOutline};
use crate::pointer::JsonPointer;

/// The query error code of a dataset whose row policies cannot all be propagated (E0H-5).
pub(super) const UNREACHABLE_ROW_ACCESS: &str = "E0H-01";

/// One declared relationship between two declared datasets, by dataset position.
struct DeclaredEdge<'model> {
    pointer: &'model JsonPointer,
    name: NormalizedName,
    from: usize,
    to: usize,
}

/// The declared relationship graph of a model, before any relationship is made unusable,
/// indexed once.
pub(super) struct DeclaredGraph<'model> {
    datasets: Vec<(&'model JsonPointer, NormalizedName)>,
    edges: Vec<DeclaredEdge<'model>>,
    /// The datasets each dataset is reached from in one step.
    incoming: Vec<Vec<usize>>,
    /// Whether each dataset has row policies or reaches a dataset that has some.
    leads_to_row_access: Vec<bool>,
}

impl<'model> DeclaredGraph<'model> {
    /// The declared graph of `model`, with the datasets that have row policies in `input`.
    pub(super) fn new(input: &'model ModelInput<'_, '_>, model: &'model ModelOutline<'_>) -> Self {
        let datasets: Vec<(&JsonPointer, NormalizedName)> = model
            .datasets
            .iter()
            .map(|dataset| (&dataset.pointer, declared(dataset.name.as_ref())))
            .collect();
        let position: HashMap<&JsonPointer, usize> = datasets
            .iter()
            .enumerate()
            .map(|(index, (pointer, _))| (*pointer, index))
            .collect();
        let index = ModelIndex::new(model);
        let dataset_at = |name: Option<&DeclaredName<'_>>| {
            name.and_then(|name| index.dataset(&name.name))
                .and_then(|dataset| position.get(&dataset.pointer).copied())
        };
        let mut edges: Vec<DeclaredEdge<'model>> = model
            .relationships
            .iter()
            .filter_map(|relationship| {
                Some(DeclaredEdge {
                    pointer: &relationship.pointer,
                    name: declared(relationship.name.as_ref()),
                    from: dataset_at(relationship.from.as_ref())?,
                    to: dataset_at(relationship.to.as_ref())?,
                })
            })
            .collect();
        edges.sort_by(|left, right| left.name.cmp(&right.name));
        let mut incoming: Vec<Vec<usize>> = datasets.iter().map(|_| Vec::new()).collect();
        for edge in &edges {
            if let Some(sources) = incoming.get_mut(edge.to) {
                sources.push(edge.from);
            }
        }
        let pointers: Vec<&JsonPointer> = datasets.iter().map(|(pointer, _)| *pointer).collect();
        let leads_to_row_access = backwards_reach(&row_access_of(input, &pointers), &incoming);
        Self {
            datasets,
            edges,
            incoming,
            leads_to_row_access,
        }
    }

    /// Every dataset that reaches a relationship `is_usable` rejects whose target is or
    /// reaches a dataset with `row_access`, with the first such relationship by normalized
    /// name, in document order.
    pub(super) fn cut_off(
        &self,
        is_usable: impl Fn(&JsonPointer) -> bool,
    ) -> Vec<(&'model JsonPointer, RowAccessCut)> {
        let mut cut: Vec<Option<&NormalizedName>> = vec![None; self.datasets.len()];
        let cutting = self.edges.iter().filter(|edge| {
            !is_usable(edge.pointer)
                && self
                    .leads_to_row_access
                    .get(edge.to)
                    .copied()
                    .unwrap_or(false)
        });
        for edge in cutting {
            let mut pending = VecDeque::from([edge.from]);
            while let Some(current) = pending.pop_front() {
                let Some(slot) = cut.get_mut(current) else {
                    continue;
                };
                if slot.is_some() {
                    continue;
                }
                *slot = Some(&edge.name);
                pending.extend(self.incoming.get(current).into_iter().flatten().copied());
            }
        }
        self.datasets
            .iter()
            .zip(cut)
            .filter_map(|((pointer, dataset), relationship)| {
                Some((
                    *pointer,
                    RowAccessCut {
                        dataset: dataset.clone(),
                        relationship: relationship?.clone(),
                    },
                ))
            })
            .collect()
    }
}

/// Whether each of `datasets` has row policies in `input`.
fn row_access_of(input: &ModelInput<'_, '_>, datasets: &[&JsonPointer]) -> Vec<bool> {
    datasets
        .iter()
        .map(|pointer| {
            input
                .extensions
                .security
                .policies
                .iter()
                .any(|policy| &policy.dataset == *pointer)
        })
        .collect()
}

/// Which datasets reach, in zero or more steps, a dataset marked in `targets`, given the
/// datasets each one is `incoming` from.
fn backwards_reach(targets: &[bool], incoming: &[Vec<usize>]) -> Vec<bool> {
    let mut reached = targets.to_vec();
    let mut pending: VecDeque<usize> = targets
        .iter()
        .enumerate()
        .filter(|(_, target)| **target)
        .map(|(index, _)| index)
        .collect();
    while let Some(current) = pending.pop_front() {
        for source in incoming.get(current).into_iter().flatten() {
            if let Some(mark) = reached.get_mut(*source)
                && !*mark
            {
                *mark = true;
                pending.push_back(*source);
            }
        }
    }
    reached
}

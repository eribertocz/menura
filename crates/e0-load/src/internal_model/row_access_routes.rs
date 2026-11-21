//! `E0H-01`: a dataset is unusable when, in the declared relationship graph, at least one
//! route from it to a dataset with `row_access` goes through an unusable relationship, even if
//! other routes stay usable, because E4-6.5 follows every route and losing one would drop a
//! row policy (E0-4.2a, E0H-2.3). A route is a directed path of relationships with no repeated
//! dataset.

use std::collections::{HashMap, VecDeque};

use super::ModelInput;
use super::index::ModelIndex;
use crate::outline::{DeclaredName, ModelOutline};
use crate::pointer::JsonPointer;

/// The query error code of a dataset whose row policies cannot all be propagated (E0H-5).
pub(super) const UNREACHABLE_ROW_ACCESS: &str = "E0H-01";

/// One declared relationship between two declared datasets, by dataset position.
struct DeclaredEdge<'model> {
    relationship: &'model JsonPointer,
    to: usize,
}

/// The declared relationship graph of a model, before any relationship is made unusable,
/// indexed once.
pub(super) struct DeclaredGraph<'model> {
    datasets: Vec<&'model JsonPointer>,
    /// The relationships that leave each dataset.
    outgoing: Vec<Vec<DeclaredEdge<'model>>>,
    /// Whether each dataset has row policies.
    row_access: Vec<bool>,
    /// Whether each dataset has row policies or reaches a dataset that has some.
    leads_to_row_access: Vec<bool>,
}

impl<'model> DeclaredGraph<'model> {
    /// The declared graph of `model`, with the datasets that have row policies in `input`.
    pub(super) fn new(input: &'model ModelInput<'_, '_>, model: &'model ModelOutline<'_>) -> Self {
        let datasets: Vec<&JsonPointer> = model
            .datasets
            .iter()
            .map(|dataset| &dataset.pointer)
            .collect();
        let position: HashMap<&JsonPointer, usize> = datasets
            .iter()
            .enumerate()
            .map(|(index, pointer)| (*pointer, index))
            .collect();
        let index = ModelIndex::new(model);
        let dataset_at = |name: Option<&DeclaredName<'_>>| {
            name.and_then(|name| index.dataset(&name.name))
                .and_then(|dataset| position.get(&dataset.pointer).copied())
        };
        let mut outgoing: Vec<Vec<DeclaredEdge<'model>>> =
            datasets.iter().map(|_| Vec::new()).collect();
        let mut incoming: Vec<Vec<usize>> = datasets.iter().map(|_| Vec::new()).collect();
        for relationship in &model.relationships {
            let (Some(from), Some(to)) = (
                dataset_at(relationship.from.as_ref()),
                dataset_at(relationship.to.as_ref()),
            ) else {
                continue;
            };
            if let (Some(edges), Some(sources)) = (outgoing.get_mut(from), incoming.get_mut(to)) {
                edges.push(DeclaredEdge {
                    relationship: &relationship.pointer,
                    to,
                });
                sources.push(from);
            }
        }
        let row_access = row_access_of(input, &datasets);
        let leads_to_row_access = backwards_reach(&row_access, &incoming);
        Self {
            datasets,
            outgoing,
            row_access,
            leads_to_row_access,
        }
    }

    /// The datasets, in document order, with a route to a dataset with `row_access` that goes
    /// through a relationship `is_usable` rejects.
    pub(super) fn cut_off(
        &self,
        is_usable: impl Fn(&JsonPointer) -> bool,
    ) -> Vec<&'model JsonPointer> {
        let mut on_route = vec![false; self.datasets.len()];
        self.datasets
            .iter()
            .enumerate()
            .filter(|(start, _)| {
                self.leads_to_row_access
                    .get(*start)
                    .copied()
                    .unwrap_or(false)
            })
            .filter(|(start, _)| {
                self.has_broken_route(*start, *start, false, &mut on_route, &is_usable)
            })
            .map(|(_, pointer)| *pointer)
            .collect()
    }

    /// Whether a route from `current`, which continues the route from `start` marked in
    /// `on_route`, reaches a dataset other than `start` with `row_access` through an unusable
    /// relationship (`broken` when the route so far has one). Only datasets that lead to
    /// `row_access` are entered.
    fn has_broken_route(
        &self,
        start: usize,
        current: usize,
        broken: bool,
        on_route: &mut [bool],
        is_usable: &impl Fn(&JsonPointer) -> bool,
    ) -> bool {
        if broken && current != start && self.row_access.get(current).copied().unwrap_or(false) {
            return true;
        }
        if let Some(mark) = on_route.get_mut(current) {
            *mark = true;
        }
        let found = self
            .outgoing
            .get(current)
            .into_iter()
            .flatten()
            .any(|edge| {
                let enterable = !on_route.get(edge.to).copied().unwrap_or(true)
                    && self
                        .leads_to_row_access
                        .get(edge.to)
                        .copied()
                        .unwrap_or(false);
                enterable
                    && self.has_broken_route(
                        start,
                        edge.to,
                        broken || !is_usable(edge.relationship),
                        on_route,
                        is_usable,
                    )
            });
        if let Some(mark) = on_route.get_mut(current) {
            *mark = false;
        }
        found
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

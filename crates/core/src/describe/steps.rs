//! Many-to-one steps between datasets and the searches over them: reachability, and the two
//! smallest paths between two datasets (DSC-3.9), found in polynomial time.
//!
//! A path never repeats a dataset (E0E-2.4). Paths are ordered by length, then by their
//! relationship names compared element by element in CON-5 order (DSC-3.9). The smallest path
//! is a breadth-first shortest path that takes, at every dataset, the step with the smallest
//! relationship name that stays on a shortest path. The second smallest is found as Yen's
//! algorithm finds it for k = 2: for each dataset of the smallest path, the smallest path that
//! leaves it by another step and avoids the datasets before it.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use menura_model::internal_model::NormalizedName;

/// One step: the relationship and the dataset it leads to (from `next`) or comes from (from
/// `previous`).
type Step<'name> = (&'name NormalizedName, &'name NormalizedName);

/// A path: its steps, in order from its first dataset.
pub(super) type Path<'name> = Vec<Step<'name>>;

/// What a search may not use.
#[derive(Default)]
struct Excluded<'name> {
    /// Datasets the path may not visit.
    datasets: BTreeSet<&'name NormalizedName>,
    /// The relationship the path may not take.
    relationship: Option<&'name NormalizedName>,
}

impl Excluded<'_> {
    fn allows(&self, (relationship, dataset): Step<'_>) -> bool {
        self.relationship != Some(relationship) && !self.datasets.contains(dataset)
    }
}

/// The steps of a graph, in both directions, each list ordered by relationship name.
#[derive(Default)]
pub(super) struct Steps<'name> {
    next: BTreeMap<&'name NormalizedName, Vec<Step<'name>>>,
    previous: BTreeMap<&'name NormalizedName, Vec<Step<'name>>>,
}

impl<'name> Steps<'name> {
    /// The graph of `edges`, each `(relationship, from, to)`.
    pub(super) fn new(
        edges: impl IntoIterator<
            Item = (
                &'name NormalizedName,
                &'name NormalizedName,
                &'name NormalizedName,
            ),
        >,
    ) -> Self {
        let mut steps = Self::default();
        for (relationship, from, to) in edges {
            steps.next.entry(from).or_default().push((relationship, to));
            steps
                .previous
                .entry(to)
                .or_default()
                .push((relationship, from));
        }
        for list in steps.next.values_mut().chain(steps.previous.values_mut()) {
            list.sort();
        }
        steps
    }

    /// The datasets `from` reaches, itself included.
    pub(super) fn reached_from(
        &self,
        from: &'name NormalizedName,
    ) -> BTreeSet<&'name NormalizedName> {
        let mut reached = BTreeSet::from([from]);
        let mut pending = vec![from];
        while let Some(dataset) = pending.pop() {
            for (_, to) in self.next.get(dataset).into_iter().flatten() {
                if reached.insert(*to) {
                    pending.push(*to);
                }
            }
        }
        reached
    }

    /// The two smallest paths from `from` to another dataset `to`, in order: none, one, or two
    /// when at least two paths exist.
    pub(super) fn two_smallest(
        &self,
        from: &'name NormalizedName,
        to: &'name NormalizedName,
    ) -> Vec<Path<'name>> {
        let Some(first) = self.smallest(from, to, &Excluded::default()) else {
            return Vec::new();
        };
        let second = self.second_smallest(from, to, &first);
        std::iter::once(first).chain(second).collect()
    }

    /// The smallest path from `from` to `to` other than `first`, the smallest one.
    fn second_smallest(
        &self,
        from: &'name NormalizedName,
        to: &'name NormalizedName,
        first: &Path<'name>,
    ) -> Option<Path<'name>> {
        let mut best: Option<Path<'name>> = None;
        let mut spur = from;
        let mut excluded = Excluded::default();
        for (index, (relationship, next)) in first.iter().enumerate() {
            excluded.relationship = Some(*relationship);
            if let Some(rest) = self.smallest(spur, to, &excluded) {
                let mut candidate: Path<'name> = first[..index].to_vec();
                candidate.extend(rest);
                if best
                    .as_ref()
                    .is_none_or(|known| is_smaller(&candidate, known))
                {
                    best = Some(candidate);
                }
            }
            excluded.datasets.insert(spur);
            spur = next;
        }
        best
    }

    /// The smallest path from `from` to `to` that `excluded` allows, `from` not excluded.
    fn smallest(
        &self,
        from: &'name NormalizedName,
        to: &'name NormalizedName,
        excluded: &Excluded<'name>,
    ) -> Option<Path<'name>> {
        let distances = self.distances_to(to, excluded);
        let mut remaining = *distances.get(from)?;
        let mut path = Vec::new();
        let mut current = from;
        while remaining > 0 {
            let step = self.next.get(current).into_iter().flatten().find(|step| {
                excluded.allows(**step) && distances.get(step.1) == Some(&(remaining - 1))
            })?;
            path.push(*step);
            current = step.1;
            remaining -= 1;
        }
        Some(path)
    }

    /// The number of steps from each dataset to `to`, over the steps `excluded` allows.
    fn distances_to(
        &self,
        to: &'name NormalizedName,
        excluded: &Excluded<'name>,
    ) -> BTreeMap<&'name NormalizedName, usize> {
        let mut distances = BTreeMap::from([(to, 0)]);
        let mut pending = VecDeque::from([to]);
        while let Some(dataset) = pending.pop_front() {
            let distance = distances.get(dataset).map_or(0, |found| found + 1);
            for step in self.previous.get(dataset).into_iter().flatten() {
                if excluded.allows(*step) && !distances.contains_key(step.1) {
                    distances.insert(step.1, distance);
                    pending.push_back(step.1);
                }
            }
        }
        distances
    }
}

/// Whether `left` comes before `right`: shorter first, then by relationship names (DSC-3.9).
fn is_smaller(left: &Path<'_>, right: &Path<'_>) -> bool {
    let left_names = left.iter().map(|(relationship, _)| *relationship);
    let right_names = right.iter().map(|(relationship, _)| *relationship);
    left.len()
        .cmp(&right.len())
        .then_with(|| left_names.cmp(right_names))
        .is_lt()
}

#[cfg(test)]
mod tests;

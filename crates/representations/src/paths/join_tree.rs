//! Join steps and the join tree of a branch (E3-3.1, E3-4.2).

use std::collections::BTreeMap;

use menura_model::internal_model::NormalizedName;

/// One step of a path: it crosses a relationship from `from` to `to` (E3-3.1).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JoinStep {
    /// The relationship crossed.
    pub relationship: NormalizedName,
    /// The dataset the step leaves.
    pub from: NormalizedName,
    /// The dataset the step reaches.
    pub to: NormalizedName,
}

/// The joins of a branch: the union of its paths (E3-4.2), in path order.
///
/// Steps are ordered from the home dataset outward: by depth, the number of steps before
/// them on their path, then by relationship name. A step whose `from` is reached by another
/// step therefore comes after that step, as joining in path order needs (E4-4).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JoinTree(Vec<JoinStep>);

impl JoinTree {
    /// The union of `paths`, each a path from the home dataset: every step once, at the
    /// smallest depth it has on any path.
    #[must_use]
    pub fn from_paths(paths: Vec<Vec<JoinStep>>) -> Self {
        let mut depths: BTreeMap<JoinStep, usize> = BTreeMap::new();
        for path in paths {
            for (depth, step) in path.into_iter().enumerate() {
                let smallest = depths.entry(step).or_insert(depth);
                *smallest = (*smallest).min(depth);
            }
        }
        let mut steps: Vec<(usize, JoinStep)> = depths
            .into_iter()
            .map(|(step, depth)| (depth, step))
            .collect();
        steps.sort();
        Self(steps.into_iter().map(|(_, step)| step).collect())
    }

    /// The steps, in path order: by depth, then by relationship name.
    #[must_use]
    pub fn steps(&self) -> &[JoinStep] {
        &self.0
    }
}

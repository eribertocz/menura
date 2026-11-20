//! Join steps and the join tree of a branch (E3-3.1, E3-4.2).

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

/// The joins of a branch: the union of its paths (E3-4.2), ordered by relationship name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JoinTree(Vec<JoinStep>);

impl JoinTree {
    /// The union of `paths`: every step of any of them once, ordered by relationship name.
    #[must_use]
    pub fn from_paths(paths: Vec<Vec<JoinStep>>) -> Self {
        let mut steps: Vec<JoinStep> = paths.into_iter().flatten().collect();
        steps.sort();
        steps.dedup();
        Self(steps)
    }

    /// The steps, ordered by relationship name.
    #[must_use]
    pub fn steps(&self) -> &[JoinStep] {
        &self.0
    }
}

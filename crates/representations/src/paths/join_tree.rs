//! Join steps, join instances and the join tree of a branch (E3-3.1, E3-4.2, E3-4.2a).

use std::collections::BTreeMap;

use menura_model::internal_model::NormalizedName;

use super::{ElementPath, QueryElement};

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

/// The position of a join instance in its join tree, from 0 (E3-4.2a).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceId(usize);

impl InstanceId {
    /// The position, from 0.
    #[must_use]
    pub fn index(self) -> usize {
        self.0
    }
}

/// One join instance: the dataset reached by one distinct path prefix (E3-4.2a).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinInstance {
    /// The last step of the prefix; its `to` is the dataset of the instance.
    pub step: JoinStep,
    /// The instance the step leaves; `None` when it leaves the home dataset.
    pub parent: Option<InstanceId>,
}

/// The joins of a branch: one instance per distinct path prefix of the union of its paths
/// (E3-4.2, E3-4.2a), in path order, with the instance each query element is bound to.
///
/// Instances are ordered from the home dataset outward: by depth, the number of steps before
/// them on their path, then by relationship name, then by the position of their parent. An
/// instance therefore comes after its parent, as joining in path order needs (E4-4.0c).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JoinTree {
    instances: Vec<JoinInstance>,
    bindings: BTreeMap<QueryElement, InstanceId>,
}

impl JoinTree {
    /// The join tree of `paths`: paths that start with the same steps share the instances
    /// of those steps, and the same dataset reached by two different step sequences is two
    /// instances. Each element is bound to the instance at the end of its own path; an
    /// element with an empty path reads the home dataset and is bound to no instance.
    #[must_use]
    pub fn from_paths(paths: Vec<ElementPath>) -> Self {
        let deepest = paths.iter().map(|path| path.steps.len()).max().unwrap_or(0);
        let mut ids: BTreeMap<Vec<JoinStep>, InstanceId> = BTreeMap::new();
        let mut instances = Vec::new();
        for depth in 0..deepest {
            let mut level: Vec<(JoinStep, Option<InstanceId>, Vec<JoinStep>)> = Vec::new();
            for path in &paths {
                let Some(prefix) = path.steps.get(..=depth) else {
                    continue;
                };
                if level.iter().any(|(_, _, known)| known.as_slice() == prefix) {
                    continue;
                }
                let parent = prefix
                    .get(..depth)
                    .and_then(|before| ids.get(before).copied());
                let step = prefix.get(depth).cloned();
                if let Some(step) = step {
                    level.push((step, parent, prefix.to_vec()));
                }
            }
            level.sort_by(|left, right| (&left.0, left.1).cmp(&(&right.0, right.1)));
            for (step, parent, prefix) in level {
                ids.insert(prefix, InstanceId(instances.len()));
                instances.push(JoinInstance { step, parent });
            }
        }
        let bindings = paths
            .into_iter()
            .filter_map(|path| {
                let id = ids.get(&path.steps).copied()?;
                Some((path.element, id))
            })
            .collect();
        Self {
            instances,
            bindings,
        }
    }

    /// The instances, in path order.
    #[must_use]
    pub fn instances(&self) -> &[JoinInstance] {
        &self.instances
    }

    /// The instance at `id`, when the tree has it.
    #[must_use]
    pub fn instance(&self, id: InstanceId) -> Option<&JoinInstance> {
        self.instances.get(id.0)
    }

    /// Whether the tree has no instance: the branch reads its home dataset only.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    /// The instance `element` is bound to; `None` when it reads the home dataset.
    #[must_use]
    pub fn bound_instance(&self, element: &QueryElement) -> Option<InstanceId> {
        self.bindings.get(element).copied()
    }

    /// The steps from the home dataset to the instance `id`, in order; empty when the tree
    /// has no such instance.
    #[must_use]
    pub fn path_to(&self, id: InstanceId) -> Vec<&JoinStep> {
        let mut steps = Vec::new();
        let mut current = self.instance(id);
        while let Some(instance) = current {
            steps.push(&instance.step);
            current = instance.parent.and_then(|parent| self.instance(parent));
        }
        steps.reverse();
        steps
    }
}

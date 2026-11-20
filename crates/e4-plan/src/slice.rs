//! The branch and the home dataset of a query of the 0.3.0 slice (VS-1).

use menura_model::internal_model::{Dataset, InternalModel};
use menura_representations::origin::{Origin, QueryLocation};
use menura_representations::paths::{Branch, Branches};

use crate::error::{InvariantViolation, PlanError};

/// The origin of an error that concerns the whole query: its document root.
pub(crate) fn query_root() -> Origin {
    Origin::Query(QueryLocation::new("", None))
}

/// The single branch of `branches`, with no join step.
///
/// # Errors
///
/// [`InvariantViolation::NoBranch`], [`InvariantViolation::SeveralBranches`] or
/// [`InvariantViolation::JoinStep`].
pub(crate) fn single_branch(branches: &Branches) -> Result<&Branch, PlanError> {
    let branch = match branches.as_slice() {
        [] => {
            return Err(PlanError::invariant(
                InvariantViolation::NoBranch,
                &query_root(),
            ));
        }
        [branch] => branch,
        [_, _, ..] => {
            return Err(PlanError::invariant(
                InvariantViolation::SeveralBranches,
                &query_root(),
            ));
        }
    };
    if branch.join_tree.steps().is_empty() {
        Ok(branch)
    } else {
        Err(PlanError::invariant(
            InvariantViolation::JoinStep,
            &query_root(),
        ))
    }
}

/// The home dataset of `branch`, which reaches no other dataset with row policies (E4-6.5).
///
/// # Errors
///
/// [`InvariantViolation::UnknownDataset`] or [`InvariantViolation::Propagation`].
pub(crate) fn home_dataset<'model>(
    branch: &Branch,
    model: &'model InternalModel,
) -> Result<&'model Dataset, PlanError> {
    let home = model
        .dataset(&branch.home)
        .ok_or_else(|| PlanError::invariant(InvariantViolation::UnknownDataset, &query_root()))?;
    let reaches_policy = model
        .parts()
        .graph
        .reachability
        .iter()
        .filter(|reach| reach.dataset == home.name)
        .flat_map(|reach| &reach.reached)
        .filter(|reached| reached.dataset != home.name)
        .any(|reached| {
            model
                .dataset(&reached.dataset)
                .is_some_and(|dataset| !dataset.row_policies.is_empty())
        });
    if reaches_policy {
        Err(PlanError::invariant(
            InvariantViolation::Propagation,
            &Origin::Model(home.location.clone()),
        ))
    } else {
        Ok(home)
    }
}

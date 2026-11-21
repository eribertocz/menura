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
    if branch.join_tree.is_empty() {
        Ok(branch)
    } else {
        Err(PlanError::invariant(
            InvariantViolation::JoinStep,
            &query_root(),
        ))
    }
}

/// The home dataset of `branch`, which E0 must not mark unusable: a dataset whose row
/// policies cannot be propagated is unusable (`E0H-01`), and planning it would read rows that
/// are not visible.
///
/// # Errors
///
/// [`InvariantViolation::UnknownDataset`] or [`InvariantViolation::UnusableDataset`].
pub(crate) fn home_dataset<'model>(
    branch: &Branch,
    model: &'model InternalModel,
) -> Result<&'model Dataset, PlanError> {
    let home = model
        .dataset(&branch.home)
        .ok_or_else(|| PlanError::invariant(InvariantViolation::UnknownDataset, &query_root()))?;
    if home.query_errors.is_empty() {
        Ok(home)
    } else {
        Err(PlanError::invariant(
            InvariantViolation::UnusableDataset,
            &Origin::Model(home.location.clone()),
        ))
    }
}

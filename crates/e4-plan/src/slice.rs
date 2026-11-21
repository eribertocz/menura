//! The branches and the home datasets of a query of the 0.4.0 track.

use menura_model::internal_model::{Dataset, InternalModel};
use menura_representations::origin::{Origin, QueryLocation};
use menura_representations::paths::{Branch, Branches};

use crate::error::{InvariantViolation, PlanError};

/// The origin of an error that concerns the whole query: its document root.
pub(crate) fn query_root() -> Origin {
    Origin::Query(QueryLocation::new("", None))
}

/// The first branch of `branches`, in branch order (E3-4.3).
///
/// # Errors
///
/// [`InvariantViolation::NoBranch`].
pub(crate) fn first_branch(branches: &Branches) -> Result<&Branch, PlanError> {
    branches.as_slice().first().ok_or_else(no_branch)
}

/// The internal error of a query without branches.
pub(crate) fn no_branch() -> PlanError {
    PlanError::invariant(InvariantViolation::NoBranch, &query_root())
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

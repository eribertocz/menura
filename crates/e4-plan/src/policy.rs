//! The row-policy predicate of the home dataset (E4-6.1 to E4-6.4, E4-6.9), evaluated by the
//! security crate (S5).

use menura_model::expression::Expression;
use menura_model::internal_model::{Dataset, SourceLocation};
use menura_representations::origin::Origin;
use menura_security::{AdmittedSecurityContext, RowFilterError, row_filter};

use crate::columns::over_scan;
use crate::error::{InvariantViolation, PlanError};

/// The row filter `G AND R` of `home` for the user of `context`, over the `Scan` columns,
/// with the location of the policies; `None` when `home` has no policy.
///
/// # Errors
///
/// [`PlanError::MissingAttribute`] (`ESEC-02`) for an applicable policy that needs an absent
/// attribute; an internal error for a condition E0 did not parse.
pub(crate) fn policy_predicate(
    home: &Dataset,
    context: &AdmittedSecurityContext,
) -> Result<Option<(Expression, SourceLocation)>, PlanError> {
    let filter = row_filter(home, context).map_err(|error| match error {
        RowFilterError::MissingAttribute { .. } => PlanError::MissingAttribute(error),
        RowFilterError::UnparsedCondition { location } => PlanError::invariant(
            InvariantViolation::UnparsedCondition,
            &Origin::Model(location),
        ),
    })?;
    let (Some(filter), Some(first)) = (filter, home.row_policies.first()) else {
        return Ok(None);
    };
    let location = first.location.clone();
    let predicate = over_scan(&filter, false)
        .map_err(|violation| PlanError::invariant(violation, &Origin::Model(location.clone())))?;
    Ok(Some((predicate, location)))
}

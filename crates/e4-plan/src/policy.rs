//! The row-policy predicate of the home dataset (E4-6.1 to E4-6.4, E4-6.9), evaluated by the
//! security crate (S5).

use std::convert::Infallible;

use menura_model::expression::Expression;
use menura_model::internal_model::{Dataset, SourceLocation};
use menura_representations::origin::Origin;
use menura_security::{AdmittedSecurityContext, RowFilterError, row_filter};

use crate::columns::over_scan;
use crate::error::{InvariantViolation, PlanError};
use crate::rewrite::rewrite;

/// The row filter `G AND R` of `home` for the user of `context`, over the `Scan` columns,
/// with the location of the policies; `None` when `home` has no policy.
///
/// # Errors
///
/// [`PlanError::MissingAttribute`] (`ESEC-02`) for an applicable policy that needs an absent
/// attribute; then [`PlanError::MissingReferenceTime`] (`EQRY-03`, E4-6.9b) for an applicable
/// policy that reads the current date or time when the query has no `reference_time`; an
/// internal error for a condition E0 did not parse.
pub(crate) fn policy_predicate(
    home: &Dataset,
    context: &AdmittedSecurityContext,
    has_reference_time: bool,
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
    if !has_reference_time && reads_now(&filter) {
        return Err(PlanError::MissingReferenceTime);
    }
    let location = first.location.clone();
    let predicate = over_scan(&filter, None)
        .map_err(|violation| PlanError::invariant(violation, &Origin::Model(location.clone())))?;
    Ok(Some((predicate, location)))
}

/// Whether `filter`, which holds the conditions of the applicable policies only (E4-6.2),
/// reads `CURRENT_DATE`, `CURRENT_TIME` or `CURRENT_TIMESTAMP`.
fn reads_now(filter: &Expression) -> bool {
    let mut found = false;
    let visited: Result<Expression, Infallible> = rewrite(filter, &mut |node| {
        found |= matches!(
            node,
            Expression::CurrentDate | Expression::CurrentTime | Expression::CurrentTimestamp
        );
        Ok(None)
    });
    drop(visited);
    found
}

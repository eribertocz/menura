//! The columns of the coverage condition of a `Join` or `SemiJoin` along a temporal
//! relationship (VAL-2.7, VAL-2.8, VAL-2.17), before the plan nodes are typed.

use menura_model::internal_model::{
    InternalModel, NormalizedName, OpenEnd, Relationship, ValidToBound,
};
use menura_representations::names::ColumnName;

use crate::error::InvariantViolation;

/// The columns of the coverage condition of a temporal `Join` or `SemiJoin`.
pub(crate) struct PlacedCoverage {
    /// The column of `from_time`, on the row side.
    pub(crate) from_time: ColumnName,
    /// The column of `valid_from`, on the version side.
    pub(crate) valid_from: ColumnName,
    /// The column of `valid_to`, on the version side.
    pub(crate) valid_to: ColumnName,
    /// Whether `valid_to` is inclusive or exclusive (VAL-1.1).
    pub(crate) valid_to_bound: ValidToBound,
    /// The `valid_to` value that reads like NULL, when declared (VAL-1.2).
    pub(crate) open_end: Option<OpenEnd>,
}

/// The coverage condition of a step along `relationship` when it is temporal: `from_time` as
/// `row_column` names it on the row side, `valid_from` and `valid_to` as `version_column`
/// names them on the version side; `None` for a plain relationship.
///
/// # Errors
///
/// [`InvariantViolation::UnknownRelationship`] for a temporal relationship whose population
/// E0 did not resolve, and the errors of the two column functions.
pub(crate) fn placed_coverage(
    relationship: &Relationship,
    model: &InternalModel,
    row_column: impl FnOnce(&NormalizedName) -> Result<ColumnName, InvariantViolation>,
    version_column: impl Fn(&NormalizedName) -> Result<ColumnName, InvariantViolation>,
) -> Result<Option<PlacedCoverage>, InvariantViolation> {
    let Some(range) = model.temporal_range(relationship) else {
        return match relationship.temporal {
            Some(_) => Err(InvariantViolation::UnknownRelationship),
            None => Ok(None),
        };
    };
    Ok(Some(PlacedCoverage {
        from_time: row_column(range.from_time)?,
        valid_from: version_column(range.valid_from)?,
        valid_to: version_column(range.valid_to)?,
        valid_to_bound: range.valid_to_bound,
        open_end: range.open_end.cloned(),
    }))
}

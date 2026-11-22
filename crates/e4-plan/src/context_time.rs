//! The query grain of a context modifier on a time dimension (E4-8.6, CTX-3.1): the one query
//! dimension whose field is the modifier's `time_dimension`, at a grain equal to or finer than
//! the modifier's `grain`.

use menura_contracts::types::common::{Grain, LogicalType};
use menura_model::internal_model::{FieldPath, InternalModel, ValueType};
use menura_representations::bound_query::{BoundDimensionKind, BoundQuery};
use menura_representations::origin::QueryLocation;

use crate::error::PlanError;

/// The time key of a query for a context modifier (E4-8.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct QueryTime {
    /// The position of the time dimension among the query dimensions, from 0.
    pub(crate) position: usize,
    /// The query grain: its `time_grains` grain, or `day` for a `date` field without one.
    pub(crate) grain: Grain,
}

/// The time key of `query` for a modifier on `time_dimension` at `grain` (E4-8.6).
///
/// # Errors
///
/// [`PlanError::ContextTime`] (`E4-02`) at `reader`, the query metric that reads the context
/// metric, when no query dimension or more than one has the field `time_dimension`, when it is
/// a `timestamp` without a time grain, or when its grain is coarser than `grain`.
pub(crate) fn query_time(
    query: &BoundQuery,
    model: &InternalModel,
    time_dimension: &FieldPath,
    grain: Grain,
    reader: &QueryLocation,
) -> Result<QueryTime, PlanError> {
    let fail = || PlanError::ContextTime(reader.clone());
    let mut found =
        query.columns().dimensions().enumerate().filter_map(
            |(position, dimension)| match &dimension.kind {
                BoundDimensionKind::Field {
                    field, time_grain, ..
                } if field == time_dimension => Some((position, *time_grain)),
                _ => None,
            },
        );
    let (Some((position, time_grain)), None) = (found.next(), found.next()) else {
        return Err(fail());
    };
    let query_grain = match time_grain {
        Some(time_grain) => time_grain,
        None if is_date(model, time_dimension) => Grain::Day,
        None => return Err(fail()),
    };
    if rank(query_grain) > rank(grain) {
        return Err(fail());
    }
    Ok(QueryTime {
        position,
        grain: query_grain,
    })
}

/// Whether the model types `field` as a `date`.
fn is_date(model: &InternalModel, field: &FieldPath) -> bool {
    model
        .dataset(&field.dataset)
        .and_then(|dataset| dataset.field(&field.field))
        .and_then(|field| field.typed.as_ref())
        .is_some_and(|typed| matches!(typed.value_type, ValueType::Logical(LogicalType::Date)))
}

/// The position of `grain` in the order `day`, `week`, `month`, `quarter`, `year`, each finer
/// than the next (E4-8.6).
fn rank(grain: Grain) -> u8 {
    match grain {
        Grain::Day => 0,
        Grain::Week => 1,
        Grain::Month => 2,
        Grain::Quarter => 3,
        Grain::Year => 4,
    }
}

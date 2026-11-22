//! The query grain of a context modifier on a time dimension (E4-8.6, CTX-3.1): the one query
//! dimension whose field is the modifier's `time_dimension`, at a grain equal to or finer than
//! the modifier's `grain`.

use menura_contracts::types::common::{Grain, LogicalType};
use menura_model::expression::{DateUnit, Expression};
use menura_model::internal_model::{FieldPath, InternalModel, ValueType};
use menura_representations::bound_query::{BoundDimensionKind, BoundQuery};

use crate::columns::field_name;

/// The time key of a query for a context modifier (E4-8.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct QueryTime {
    /// The position of the time dimension among the query dimensions, from 0.
    pub(crate) position: usize,
    /// The query grain: its `time_grains` grain, the unit of its `DATE_TRUNC` in the SQL form,
    /// or `day` for a `date` field without one.
    pub(crate) grain: Grain,
}

/// The time key of `query` for a modifier on `time_dimension` at `grain` (E4-8.6). `None`,
/// which is `E4-02` at the query metric that reads the context metric, when no query dimension
/// or more than one reads `time_dimension`, when it is a `timestamp` without a time grain, or
/// when its grain is coarser than `grain`.
pub(crate) fn query_time(
    query: &BoundQuery,
    model: &InternalModel,
    time_dimension: &FieldPath,
    grain: Grain,
) -> Option<QueryTime> {
    let mut found = query
        .columns()
        .dimensions()
        .enumerate()
        .filter_map(|(position, dimension)| {
            time_key(&dimension.kind, time_dimension).map(|key| (position, key))
        });
    let (Some((position, key)), None) = (found.next(), found.next()) else {
        return None;
    };
    let query_grain = match key {
        TimeKey::Grain(Some(time_grain)) => time_grain,
        TimeKey::Grain(None) if is_date(model, time_dimension) => Grain::Day,
        TimeKey::Grain(None) | TimeKey::FinerThanDay => return None,
    };
    (rank(query_grain) <= rank(grain)).then_some(QueryTime {
        position,
        grain: query_grain,
    })
}

/// How a query dimension reads the time dimension.
enum TimeKey {
    /// At this time grain, or without one.
    Grain(Option<Grain>),
    /// At a unit finer than a day, which is no query grain.
    FinerThanDay,
}

/// How `dimension` reads `time_dimension`; `None` when it does not. In the SQL form,
/// `DATE_TRUNC('<unit>', <time_dimension>)` counts as that field at that grain, as E4-7.8
/// states for `over` (maintainer decision on #493).
fn time_key(dimension: &BoundDimensionKind, time_dimension: &FieldPath) -> Option<TimeKey> {
    match dimension {
        BoundDimensionKind::Field {
            field, time_grain, ..
        } => (field == time_dimension).then_some(TimeKey::Grain(*time_grain)),
        BoundDimensionKind::Expression {
            dataset,
            expression: Expression::DateTrunc { unit, operand },
        } if dataset == &time_dimension.dataset && reads(operand, time_dimension) => {
            Some(unit_key(*unit))
        }
        BoundDimensionKind::Expression { .. } => None,
    }
}

/// Whether `operand` is a reference to the field of `time_dimension`.
fn reads(operand: &Expression, time_dimension: &FieldPath) -> bool {
    matches!(operand, Expression::Reference(reference)
        if field_name(&reference.name) == time_dimension.field)
}

/// The time key a `DATE_TRUNC` unit gives.
fn unit_key(unit: DateUnit) -> TimeKey {
    match unit {
        DateUnit::Year => TimeKey::Grain(Some(Grain::Year)),
        DateUnit::Quarter => TimeKey::Grain(Some(Grain::Quarter)),
        DateUnit::Month => TimeKey::Grain(Some(Grain::Month)),
        DateUnit::Week => TimeKey::Grain(Some(Grain::Week)),
        DateUnit::Day => TimeKey::Grain(Some(Grain::Day)),
        DateUnit::Hour | DateUnit::Minute | DateUnit::Second => TimeKey::FinerThanDay,
    }
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

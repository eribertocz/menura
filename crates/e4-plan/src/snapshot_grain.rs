//! `E4-03`: a query grain of a semi-additive `over` finer than its `snapshot_grain` (ADD-4.9,
//! E4-7.8), since a snapshot cannot be split.

use menura_contracts::types::common::{Grain, LogicalType};
use menura_model::expression::{DateUnit, Expression};
use menura_model::internal_model::FieldPath;
use menura_representations::bound_query::{BoundDimension, BoundDimensionKind, BoundQuery};
use menura_representations::origin::QueryLocation;

use crate::columns::field_name;
use crate::error::PlanError;

#[cfg(test)]
mod tests;

/// The grain at which a query dimension reads a time `over` (E4-7.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverGrain {
    /// A time grain: the dimension's own, or `day` for a plain `date`.
    Grain(Grain),
    /// Finer than every grain: the source precision of a plain `timestamp`, or a truncation
    /// of the SQL form to a unit below `day`.
    FinerThanDay,
}

impl OverGrain {
    /// The grain of a dimension of `over` whose type is `over_type`, with the dimension's
    /// `time_grain`; `None` when `over_type` is not a time type.
    pub(crate) fn of(time_grain: Option<Grain>, over_type: &LogicalType) -> Option<Self> {
        match (time_grain, over_type) {
            (Some(grain), LogicalType::Date | LogicalType::Timestamp { .. }) => {
                Some(Self::Grain(grain))
            }
            (None, LogicalType::Date) => Some(Self::Grain(Grain::Day)),
            (None, LogicalType::Timestamp { .. }) => Some(Self::FinerThanDay),
            _ => None,
        }
    }

    /// Whether a snapshot at `snapshot_grain` would be split at this grain (ADD-4.9).
    pub(crate) fn is_finer_than(self, snapshot_grain: Grain) -> bool {
        match self {
            Self::FinerThanDay => true,
            Self::Grain(grain) => fineness(grain) < fineness(snapshot_grain),
        }
    }
}

/// The time `over` of one semi-additive metric of the query.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SnapshotAxis<'model> {
    /// The `over` field.
    pub(crate) over: &'model FieldPath,
    /// Its type.
    pub(crate) over_type: &'model LogicalType,
    /// The `snapshot_grain`.
    pub(crate) snapshot_grain: Grain,
}

/// `E4-03` at the first dimension of `query`, in query order, that splits the snapshots of one
/// of `axes`: located at its `time_grains` entry, else at the dimension, which in the SQL form
/// is its `SELECT` item (E4-7.8, E2-5.3).
///
/// # Errors
///
/// [`PlanError::SplitSnapshot`] for that dimension.
pub(crate) fn check_snapshot_grains(
    query: &BoundQuery,
    axes: &[SnapshotAxis<'_>],
) -> Result<(), PlanError> {
    let split = query
        .columns()
        .dimensions()
        .find(|dimension| axes.iter().any(|axis| splits(&dimension.kind, axis)));
    match split {
        Some(dimension) => Err(PlanError::SplitSnapshot(location(dimension).clone())),
        None => Ok(()),
    }
}

/// Whether `dimension` reads the `over` of `axis` at a grain finer than its `snapshot_grain`.
fn splits(dimension: &BoundDimensionKind, axis: &SnapshotAxis<'_>) -> bool {
    over_grain(dimension, axis).is_some_and(|grain| grain.is_finer_than(axis.snapshot_grain))
}

/// The grain at which `dimension` reads the `over` of `axis`; `None` when it does not read it.
pub(crate) fn over_grain(
    dimension: &BoundDimensionKind,
    axis: &SnapshotAxis<'_>,
) -> Option<OverGrain> {
    match dimension {
        BoundDimensionKind::Field {
            field, time_grain, ..
        } if field == axis.over => OverGrain::of(*time_grain, axis.over_type),
        BoundDimensionKind::Expression {
            dataset,
            expression,
        } if dataset == &axis.over.dataset => sql_grain(expression, axis),
        BoundDimensionKind::Field { .. } | BoundDimensionKind::Expression { .. } => None,
    }
}

/// The grain of a dimension of the SQL form: `over` itself, or `DATE_TRUNC('<unit>', over)`,
/// which counts as that grain (E4-7.8).
fn sql_grain(expression: &Expression, axis: &SnapshotAxis<'_>) -> Option<OverGrain> {
    let reads_over = |operand: &Expression| match operand {
        Expression::Reference(reference) => field_name(&reference.name) == axis.over.field,
        _ => false,
    };
    match expression {
        Expression::DateTrunc { unit, operand } if reads_over(operand) => {
            OverGrain::of(None, axis.over_type).map(|_| unit_grain(*unit))
        }
        operand if reads_over(operand) => OverGrain::of(None, axis.over_type),
        _ => None,
    }
}

/// The grain a `DATE_TRUNC` unit counts as.
fn unit_grain(unit: DateUnit) -> OverGrain {
    match unit {
        DateUnit::Year => OverGrain::Grain(Grain::Year),
        DateUnit::Quarter => OverGrain::Grain(Grain::Quarter),
        DateUnit::Month => OverGrain::Grain(Grain::Month),
        DateUnit::Week => OverGrain::Grain(Grain::Week),
        DateUnit::Day => OverGrain::Grain(Grain::Day),
        DateUnit::Hour | DateUnit::Minute | DateUnit::Second => OverGrain::FinerThanDay,
    }
}

/// Where `E4-03` is located for `dimension`: its `time_grains` entry, else the dimension.
fn location(dimension: &BoundDimension) -> &QueryLocation {
    match &dimension.kind {
        BoundDimensionKind::Field {
            time_grain_origin: Some(origin),
            ..
        } => origin,
        BoundDimensionKind::Field { .. } | BoundDimensionKind::Expression { .. } => {
            &dimension.origin
        }
    }
}

/// The rank of `grain` from the finest, `day`, to the coarsest, `year`.
fn fineness(grain: Grain) -> u8 {
    match grain {
        Grain::Day => 0,
        Grain::Week => 1,
        Grain::Month => 2,
        Grain::Quarter => 3,
        Grain::Year => 4,
    }
}

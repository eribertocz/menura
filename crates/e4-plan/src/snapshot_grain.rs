//! `E4-03`: a query grain of a semi-additive `over` finer than its `snapshot_grain` (ADD-4.9,
//! E4-7.8), since a snapshot cannot be split.

use menura_contracts::types::common::{Grain, LogicalType};
use menura_model::internal_model::FieldPath;
use menura_representations::bound_query::{BoundDimensionKind, BoundQuery};

#[cfg(test)]
mod tests;

/// The grain at which a query dimension reads a time `over` (E4-7.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverGrain {
    /// A time grain: the dimension's own, or `day` for a plain `date`.
    Grain(Grain),
    /// The source precision of a plain `timestamp`, finer than every grain.
    SourcePrecision,
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
            (None, LogicalType::Timestamp { .. }) => Some(Self::SourcePrecision),
            _ => None,
        }
    }

    /// Whether a snapshot at `snapshot_grain` would be split at this grain (ADD-4.9).
    pub(crate) fn is_finer_than(self, snapshot_grain: Grain) -> bool {
        match self {
            Self::SourcePrecision => true,
            Self::Grain(grain) => fineness(grain) < fineness(snapshot_grain),
        }
    }
}

/// The position of the first dimension of `query`, in output order, that reads `over` at a
/// grain finer than `snapshot_grain`, `over` having type `over_type`.
pub(crate) fn first_finer_dimension(
    query: &BoundQuery,
    over: &FieldPath,
    over_type: &LogicalType,
    snapshot_grain: Grain,
) -> Option<usize> {
    let dimensions = query
        .columns()
        .dimensions()
        .map(|dimension| &dimension.kind);
    first_finer(dimensions, over, over_type, snapshot_grain)
}

/// The position of the first of `dimensions` that reads `over` at a grain finer than
/// `snapshot_grain`.
fn first_finer<'query>(
    mut dimensions: impl Iterator<Item = &'query BoundDimensionKind>,
    over: &FieldPath,
    over_type: &LogicalType,
    snapshot_grain: Grain,
) -> Option<usize> {
    dimensions.position(|dimension| match dimension {
        BoundDimensionKind::Field { field, time_grain } if field == over => {
            OverGrain::of(*time_grain, over_type)
                .is_some_and(|grain| grain.is_finer_than(snapshot_grain))
        }
        BoundDimensionKind::Field { .. } | BoundDimensionKind::Expression { .. } => false,
    })
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

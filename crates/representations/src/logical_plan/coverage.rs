//! The coverage condition of a `Join` or `SemiJoin` along a temporal relationship (VAL-2.7,
//! VAL-2.8): a row matches a version only when the version covers the row's `from_time`.

use menura_contracts::types::common::{LogicalType, TimestampPrecision};

use crate::check_shapes::VersionRange;
use crate::names::ColumnName;

/// What a temporal relationship adds to the key equalities of a `Join` or `SemiJoin`: the
/// version must cover the `from_time` value of the row (VAL-2.7, VAL-2.8). The condition reads
/// no query value, so it adds no parameter (VAL-2.21).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalCoverage {
    /// The column of the row side (the left input of a `Join`, the input of a `SemiJoin`) that
    /// holds `from_time`.
    pub from_time: ColumnName,
    /// The range of each version, over the columns of the version side.
    pub range: VersionRange,
    /// The type the three values are compared as (VAL-2.9); see [`comparison_type`].
    pub compared_as: LogicalType,
}

/// The type in which `from_time`, `valid_from` and `valid_to` are compared (VAL-2.9): E0.f
/// checked that they are all `date`, or all `timestamp` with the same `with_time_zone`
/// (VAL-2.5); timestamps are compared at the finest of their precisions, so that a value of a
/// coarser precision is compared exactly, never rounded.
#[must_use]
pub fn comparison_type(types: [&LogicalType; 3]) -> LogicalType {
    let finest = types
        .iter()
        .filter_map(|logical_type| match logical_type {
            LogicalType::Timestamp { precision, .. } => Some(*precision),
            _ => None,
        })
        .max_by_key(|precision| fraction_digits(*precision));
    match (types[0], finest) {
        (LogicalType::Timestamp { with_time_zone, .. }, Some(precision)) => {
            LogicalType::Timestamp {
                precision,
                with_time_zone: *with_time_zone,
            }
        }
        (first, _) => first.clone(),
    }
}

/// The number of fraction digits a timestamp of `precision` stores.
fn fraction_digits(precision: TimestampPrecision) -> u8 {
    match precision {
        TimestampPrecision::S => 0,
        TimestampPrecision::Ms => 3,
        TimestampPrecision::Us => 6,
        TimestampPrecision::Ns => 9,
    }
}

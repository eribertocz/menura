//! The day a time value belongs to (POP-3.5, E6-7.6): a `date` as it is, a `timestamp with
//! time zone` read in the query time zone. And the periods of a grain over dates: their start,
//! the date a number of periods later, and the series of period starts between two dates.

use menura_contracts::types::common::{Grain, LogicalType};
use menura_representations::logical_plan::PlanNode;

use crate::error::{GenerateError, InvariantViolation};
use crate::expression::Columns;
use crate::steps::{StepWriter, error};
use crate::types::digits;

impl<'input> StepWriter<'input> {
    /// The day of `value`, a time value of `logical_type` read by `node` from `input`, as a
    /// `Date32`: `toDate32(toTimeZone(x, tz))` for a timestamp with time zone (E6-7.6).
    ///
    /// # Errors
    ///
    /// [`InvariantViolation::UnsupportedType`] for any other type.
    pub(crate) fn day_sql(
        &mut self,
        node: &'input PlanNode,
        input: &'input PlanNode,
        value: &str,
        logical_type: &LogicalType,
    ) -> Result<String, GenerateError> {
        match logical_type {
            LogicalType::Date => Ok(value.to_owned()),
            LogicalType::Timestamp {
                with_time_zone: true,
                ..
            } => {
                let zone = self
                    .writer(node, Columns::Input(input.schema()))
                    .time_zone();
                Ok(format!("toDate32(toTimeZone({value}, {zone}))"))
            }
            _ => Err(error(InvariantViolation::UnsupportedType, node)),
        }
    }
}

impl<'input> StepWriter<'input> {
    /// The first instant of the day `start`, a `Date32`, as a value of `logical_type`: the
    /// date itself, or its midnight in the query time zone as a UTC timestamp of the type's
    /// precision.
    ///
    /// # Errors
    ///
    /// [`InvariantViolation::UnsupportedType`] for any other type.
    pub(crate) fn start_sql(
        &mut self,
        node: &'input PlanNode,
        start: &str,
        logical_type: &LogicalType,
    ) -> Result<String, GenerateError> {
        match logical_type {
            LogicalType::Date => Ok(start.to_owned()),
            LogicalType::Timestamp {
                precision,
                with_time_zone: true,
            } => {
                let zone = self.writer(node, Columns::Input(node.schema())).time_zone();
                Ok(format!(
                    "toTimeZone(toDateTime64({start}, {}, {zone}), 'UTC')",
                    digits(*precision)
                ))
            }
            _ => Err(error(InvariantViolation::UnsupportedType, node)),
        }
    }
}

/// The first day of the period at `grain` that contains the date `date`, as a `Date32`
/// (E6-7.6): ISO weeks start on Monday.
pub(crate) fn period_start(grain: Grain, date: &str) -> String {
    match grain {
        Grain::Day => format!("toDate32({date})"),
        Grain::Week => format!("toDate32(toMonday({date}))"),
        Grain::Month => format!("toDate32(toStartOfMonth({date}))"),
        Grain::Quarter => format!("toDate32(toStartOfQuarter({date}))"),
        Grain::Year => format!("toDate32(toStartOfYear({date}))"),
    }
}

/// The date `count` periods at `grain` after the date `date`.
pub(crate) fn periods_after(grain: Grain, date: &str, count: &str) -> String {
    let function = match grain {
        Grain::Day => "addDays",
        Grain::Week => "addWeeks",
        Grain::Month => "addMonths",
        Grain::Quarter => "addQuarters",
        Grain::Year => "addYears",
    };
    format!("{function}({date}, {count})")
}

/// The array of the dates from `first`, the start of a period at `grain`, by steps of one
/// period, up to `last` included: empty when `last` is before `first`. `first` is read
/// through `assumeNotNull`, since an array cannot be `Nullable`; a row whose `first` is NULL
/// is the caller's to remove.
pub(crate) fn period_series(grain: Grain, first: &str, last: &str) -> String {
    let (unit, step) = match grain {
        Grain::Day => ("day", 1),
        Grain::Week => ("day", 7),
        Grain::Month => ("month", 1),
        Grain::Quarter => ("month", 3),
        Grain::Year => ("month", 12),
    };
    let first = format!("assumeNotNull({first})");
    let difference = format!("dateDiff('{unit}', {first}, {last})");
    let count =
        format!("toUInt64(if(ifNull({difference} < 0, 1), 0, intDiv({difference}, {step}) + 1))");
    format!(
        "arrayMap(menura_offset -> {}, range({count}))",
        periods_after(grain, &first, "menura_offset")
    )
}

/// The number of period boundaries at `grain` from the date `start` to the date `end`, ISO
/// weeks starting on Monday.
pub(crate) fn periods_between(grain: Grain, start: &str, end: &str) -> String {
    match grain {
        Grain::Week => format!("intDiv(dateDiff('day', toMonday({start}), toMonday({end})), 7)"),
        Grain::Day => format!("dateDiff('day', {start}, {end})"),
        Grain::Month => format!("dateDiff('month', {start}, {end})"),
        Grain::Quarter => format!("dateDiff('quarter', {start}, {end})"),
        Grain::Year => format!("dateDiff('year', {start}, {end})"),
    }
}

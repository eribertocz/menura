//! The day a time value belongs to (POP-3.5, E6-5.2): a `date` as it is, a `timestamp` cast
//! to its date, a `timestamp with time zone` converted to the query time zone first.

use menura_contracts::types::common::{Grain, LogicalType};
use menura_representations::logical_plan::PlanNode;

use crate::error::{GenerateError, InvariantViolation};
use crate::expression::Columns;
use crate::steps::{StepWriter, error};
use crate::types::physical_type;

impl<'input> StepWriter<'input> {
    /// The day of `value`, of type `logical_type`, written in `node` over the columns of
    /// `input`. The time-zone parameter is bound only when the value needs it (E6-4.2).
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
                Ok(format!("CAST(({value}) AT TIME ZONE {zone} AS DATE)"))
            }
            LogicalType::Timestamp { .. } => Ok(format!("CAST({value} AS DATE)")),
            _ => Err(error(InvariantViolation::UnsupportedType, node)),
        }
    }
}

impl<'input> StepWriter<'input> {
    /// `start`, a `date`, as a value of `logical_type` in `node`: the date, or the midnight
    /// that starts it, in the query time zone for `timestamp with time zone` (E6-4.10).
    pub(crate) fn start_sql(
        &mut self,
        node: &'input PlanNode,
        start: &str,
        logical_type: &LogicalType,
    ) -> Result<String, GenerateError> {
        let physical = physical_type(logical_type)
            .ok_or_else(|| error(InvariantViolation::UnsupportedType, node))?;
        match logical_type {
            LogicalType::Date => Ok(start.to_owned()),
            LogicalType::Timestamp {
                with_time_zone: true,
                ..
            } => {
                let zone = self.writer(node, Columns::Input(node.schema())).time_zone();
                Ok(format!(
                    "CAST(CAST({start} AS TIMESTAMP) AT TIME ZONE {zone} AS {physical})"
                ))
            }
            LogicalType::Timestamp { .. } => Ok(format!("CAST({start} AS {physical})")),
            _ => Err(error(InvariantViolation::UnsupportedType, node)),
        }
    }
}

/// The length of a period at `grain`, as an `INTERVAL` (E6-5.2).
pub(crate) fn grain_interval(grain: Grain) -> &'static str {
    match grain {
        Grain::Day => "INTERVAL 1 DAY",
        Grain::Week => "INTERVAL 7 DAY",
        Grain::Month => "INTERVAL 1 MONTH",
        Grain::Quarter => "INTERVAL 3 MONTH",
        Grain::Year => "INTERVAL 1 YEAR",
    }
}

/// The `date_trunc` name of `grain`, a literal from a closed set (E6-5.2, E6-4.11).
pub(crate) fn grain_name(grain: Grain) -> &'static str {
    match grain {
        Grain::Day => "day",
        Grain::Week => "week",
        Grain::Month => "month",
        Grain::Quarter => "quarter",
        Grain::Year => "year",
    }
}

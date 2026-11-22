//! When an entity is expected in a period (POP-3.1, POP-3.2): the join condition between a row
//! of the population's input and a period.

use menura_contracts::types::common::{Grain, LogicalType};
use menura_model::internal_model::PartialPeriod;
use menura_representations::logical_plan::{Membership, PlanNode};
use menura_representations::names::ColumnName;

use super::ROW;
use crate::error::{GenerateError, InvariantViolation};
use crate::identifiers::quote;
use crate::parameters::SharedValue;
use crate::steps::{StepWriter, error};

impl StepWriter<'_> {
    /// The condition under which the row covers `period`, the start of a period at `grain`.
    ///
    /// `validity` reads inclusive days, a NULL `valid_to` being open-ended and a NULL
    /// `valid_from` never matching: `any_day` needs one day of the period, `full_period` every
    /// day, `period_end` the last day (POP-3.1). `periodic` needs the row's period, truncated
    /// to `grain`, to be `period`; a NULL period never matches (POP-3.2). A `timestamp` value
    /// belongs to its day in the query time zone (E6-5.2).
    pub(super) fn covers(
        &mut self,
        node: &PlanNode,
        input: &PlanNode,
        membership: &Membership,
        period: &str,
        grain: Grain,
    ) -> Result<String, GenerateError> {
        match membership {
            Membership::Validity {
                valid_from,
                valid_to,
                partial_period,
            } => {
                let from = self.day_of(node, input, valid_from)?;
                let to = self.day_of(node, input, valid_to)?;
                let last = last_day(period, grain);
                let (from_bound, to_bound) = match partial_period {
                    PartialPeriod::AnyDay => (last.as_str(), period),
                    PartialPeriod::FullPeriod => (period, last.as_str()),
                    PartialPeriod::PeriodEnd => (last.as_str(), last.as_str()),
                };
                Ok(format!(
                    "{from} <= {from_bound} AND ({to} IS NULL OR {to} >= {to_bound})"
                ))
            }
            Membership::Periodic {
                period: row_period, ..
            } => {
                let row_period = self.day_of(node, input, row_period)?;
                Ok(format!(
                    "CAST(date_trunc('{}', {row_period}) AS DATE) = {period}",
                    grain_name(grain)
                ))
            }
        }
    }

    /// The day of the column `name` of the row: a `date` as it is, a `timestamp` cast to its
    /// date, a `timestamp with time zone` converted to the query time zone first (E6-5.2).
    fn day_of(
        &mut self,
        node: &PlanNode,
        input: &PlanNode,
        name: &ColumnName,
    ) -> Result<String, GenerateError> {
        let value = format!("{ROW}.{}", quote(name.as_str()));
        match column_type(node, input, name)? {
            LogicalType::Date => Ok(value),
            LogicalType::Timestamp {
                with_time_zone: true,
                ..
            } => {
                let zone = self.time_zone();
                Ok(format!("CAST({value} AT TIME ZONE {zone} AS DATE)"))
            }
            LogicalType::Timestamp { .. } => Ok(format!("CAST({value} AS DATE)")),
            _ => Err(error(InvariantViolation::UnsupportedType, node)),
        }
    }

    /// The query time zone, as a typed parameter (E6-4.2c).
    fn time_zone(&mut self) -> String {
        let name = self.environment.time.time_zone.as_str().to_owned();
        let placeholder = self.parameters.bind_shared(
            SharedValue::TimeZone,
            LogicalType::Text,
            serde_json::Value::String(name),
        );
        format!("CAST({placeholder} AS VARCHAR)")
    }
}

/// The type of the column `name` of `input`.
pub(super) fn column_type<'plan>(
    node: &PlanNode,
    input: &'plan PlanNode,
    name: &ColumnName,
) -> Result<&'plan LogicalType, GenerateError> {
    input
        .schema()
        .columns()
        .iter()
        .find(|column| &column.name == name)
        .map(|column| &column.logical_type)
        .ok_or_else(|| error(InvariantViolation::UnknownColumn, node))
}

/// The last day of the period at `grain` that starts on `period`.
fn last_day(period: &str, grain: Grain) -> String {
    let length = match grain {
        Grain::Day => return period.to_owned(),
        Grain::Week => "7 DAY",
        Grain::Month => "1 MONTH",
        Grain::Quarter => "3 MONTH",
        Grain::Year => "1 YEAR",
    };
    format!("CAST({period} + INTERVAL {length} - INTERVAL 1 DAY AS DATE)")
}

/// The `date_trunc` name of `grain` (E6-5.2).
pub(crate) fn grain_name(grain: Grain) -> &'static str {
    match grain {
        Grain::Day => "day",
        Grain::Week => "week",
        Grain::Month => "month",
        Grain::Quarter => "quarter",
        Grain::Year => "year",
    }
}

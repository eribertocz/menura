//! When an entity is expected in a period (POP-3.1, POP-3.2): the join condition between a row
//! of the population's input and a period.

use menura_contracts::types::common::{Grain, LogicalType};
use menura_model::internal_model::PartialPeriod;
use menura_representations::logical_plan::{Membership, PlanNode};
use menura_representations::names::ColumnName;

use super::ROW;
use crate::day::{grain_interval, grain_name};
use crate::error::{GenerateError, InvariantViolation};
use crate::identifiers::quote;
use crate::steps::{StepWriter, error};

impl<'input> StepWriter<'input> {
    /// The condition under which the row covers `period`, the start of a period at `grain`.
    ///
    /// `validity` reads inclusive days, a NULL `valid_to` being open-ended and a NULL
    /// `valid_from` never matching: `any_day` needs one day of the period, `full_period` every
    /// day, `period_end` the last day (POP-3.1). `periodic` needs the row's period, truncated
    /// to `grain`, to be `period`; a NULL period never matches (POP-3.2). A `timestamp` value
    /// belongs to its day in the query time zone (E6-5.2).
    pub(crate) fn covers(
        &mut self,
        node: &'input PlanNode,
        input: &'input PlanNode,
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

    /// The day of the column `name` of the row (POP-3.5).
    fn day_of(
        &mut self,
        node: &'input PlanNode,
        input: &'input PlanNode,
        name: &ColumnName,
    ) -> Result<String, GenerateError> {
        let value = format!("{ROW}.{}", quote(name.as_str()));
        let logical_type = column_type(node, input, name)?;
        self.day_sql(node, input, &value, logical_type)
    }
}

/// The type of the column `name` of `input`.
pub(crate) fn column_type<'plan>(
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
    if grain == Grain::Day {
        return period.to_owned();
    }
    format!(
        "CAST({period} + {} - INTERVAL 1 DAY AS DATE)",
        grain_interval(grain)
    )
}

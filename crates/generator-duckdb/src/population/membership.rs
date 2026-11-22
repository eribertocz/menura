//! When an entity is expected in a period (POP-3.1, POP-3.2): the join condition between a row
//! of the population's input and a period, for `date` columns.

use menura_contracts::types::common::{Grain, LogicalType};
use menura_model::internal_model::PartialPeriod;
use menura_representations::logical_plan::{Membership, PlanNode};
use menura_representations::names::ColumnName;

use super::ROW;
use crate::error::{GenerateError, InvariantViolation};
use crate::identifiers::quote;
use crate::steps::error;

/// The condition under which the row covers `period`, the start of a period at `grain`.
///
/// `validity` reads inclusive days, a NULL `valid_to` being open-ended: `any_day` needs one day
/// of the period, `full_period` every day, `period_end` the last day (POP-3.1). `periodic`
/// needs the row's period, truncated to `grain`, to be `period` (POP-3.2).
pub(super) fn covers(
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
            let from = date_column(node, input, valid_from)?;
            let to = date_column(node, input, valid_to)?;
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
            let row_period = date_column(node, input, row_period)?;
            Ok(format!(
                "CAST(date_trunc('{}', {row_period}) AS DATE) = {period}",
                grain_name(grain)
            ))
        }
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

/// The column `name` of the row, which must be a `date`.
fn date_column(
    node: &PlanNode,
    input: &PlanNode,
    name: &ColumnName,
) -> Result<String, GenerateError> {
    match column_type(node, input, name)? {
        LogicalType::Date => Ok(format!("{ROW}.{}", quote(name.as_str()))),
        _ => Err(error(InvariantViolation::UnsupportedType, node)),
    }
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
fn grain_name(grain: Grain) -> &'static str {
    match grain {
        Grain::Day => "day",
        Grain::Week => "week",
        Grain::Month => "month",
        Grain::Quarter => "quarter",
        Grain::Year => "year",
    }
}

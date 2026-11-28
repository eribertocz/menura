//! When an entity is expected in a period (POP-3.1, POP-3.2): the join condition between a row
//! of the population's input and a period.

use menura_contracts::types::common::{Grain, LogicalType};
use menura_model::internal_model::{OpenEnd, PartialPeriod, ValidToBound};
use menura_representations::logical_plan::{Membership, PlanNode};
use menura_representations::names::ColumnName;

use super::ROW;
use crate::day::{period_start, periods_after};
use crate::error::{GenerateError, InvariantViolation};
use crate::identifiers::quote;
use crate::steps::{StepWriter, error};
use crate::types::digits;
use crate::values::{date_text, text_sql};

impl<'input> StepWriter<'input> {
    /// The condition under which the row covers `period`, the start of a period at `grain`.
    ///
    /// `validity` reads days, a NULL `valid_to` or one equal to `open_end` being open-ended
    /// (VAL-1.2) and a NULL `valid_from` never matching: `any_day` needs one day of the period,
    /// `full_period` every day, `period_end` the last day (POP-3.1). An exclusive `valid_to`
    /// v ends on the day of the last value before v (VAL-1.3), which is on or after a day d
    /// exactly when v is after the first instant of d. `periodic` needs the row's period, truncated
    /// to `grain`, to be `period`; a NULL period never matches (POP-3.2). A `timestamp` value
    /// belongs to its day in the query time zone (E6-7.6).
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
                valid_to_bound,
                open_end,
            } => {
                let from = self.day_of(node, input, valid_from)?;
                let last = last_day(period, grain);
                let (from_bound, to_bound) = match partial_period {
                    PartialPeriod::AnyDay => (last.as_str(), period),
                    PartialPeriod::FullPeriod => (period, last.as_str()),
                    PartialPeriod::PeriodEnd => (last.as_str(), last.as_str()),
                };
                let to_type = column_type(node, input, valid_to)?;
                let row_to = format!("{ROW}.{}", quote(valid_to.as_str()));
                let to = open_ended(node, &row_to, open_end.as_ref(), to_type)?;
                let end = match valid_to_bound {
                    ValidToBound::Inclusive => {
                        let day = self.day_sql(node, input, &to, to_type)?;
                        format!("{day} IS NULL OR {day} >= {to_bound}")
                    }
                    ValidToBound::Exclusive => {
                        let start = self.start_sql(node, to_bound, to_type)?;
                        format!("{to} IS NULL OR {to} > {start}")
                    }
                };
                Ok(format!("{from} <= {from_bound} AND ({end})"))
            }
            Membership::Periodic {
                period: row_period, ..
            } => {
                let row_period = self.day_of(node, input, row_period)?;
                Ok(format!("{} = {period}", period_start(grain, &row_period)))
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

/// `value`, a `valid_to` of `logical_type`, as NULL when it equals `open_end` (VAL-1.2): a
/// model constant, written as a literal of the type of `valid_to` (E6-4.11, E6-7.2).
///
/// # Errors
///
/// [`InvariantViolation::UnsupportedType`] for a type other than `date` and `timestamp` with
/// time zone, which `ClickHouse` cannot express (E6-7.12).
pub(crate) fn open_ended(
    node: &PlanNode,
    value: &str,
    open_end: Option<&OpenEnd>,
    logical_type: &LogicalType,
) -> Result<String, GenerateError> {
    let constant = match (open_end, logical_type) {
        (None, _) => return Ok(value.to_owned()),
        (Some(OpenEnd::Date(date)), LogicalType::Date) => {
            format!("toDate32({})", text_sql(&date_text(*date)))
        }
        (
            Some(OpenEnd::Timestamp(timestamp)),
            LogicalType::Timestamp {
                precision,
                with_time_zone: true,
            },
        ) if timestamp.with_time_zone => format!(
            "toDateTime64({}, {}, 'UTC')",
            text_sql(&format!("{} {}", date_text(timestamp.date), timestamp.time)),
            digits(*precision)
        ),
        _ => return Err(error(InvariantViolation::UnsupportedType, node)),
    };
    Ok(format!("nullIf({value}, {constant})"))
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
    format!("addDays({}, -1)", periods_after(grain, period, "1"))
}

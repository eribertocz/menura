//! The step of a `Periods` node (E4-3.14, E6-4.14): one row per period start at a grain, from
//! the period of `from` to the period of `to`, both included.

use menura_contracts::types::common::{Grain, LogicalType};
use menura_model::expression::{DateValue, Expression, Literal};
use menura_representations::logical_plan::{Operation, PeriodShift, PlanNode};
use serde_json::Value;

use crate::body::SelectBody;
use crate::day::{grain_interval, grain_name};
use crate::error::{GenerateError, InvariantViolation};
use crate::expression::Columns;
use crate::identifiers::quote;
use crate::steps::{StepWriter, error};
use crate::values::date_text;

/// The synthesized column of a period start (E6-4.12).
const START: &str = "\"menura.start\"";

impl<'input> StepWriter<'input> {
    /// The body of `node`, a `Periods`; any other node is not one this function generates.
    pub(crate) fn periods(&mut self, node: &'input PlanNode) -> Result<SelectBody, GenerateError> {
        let Operation::Periods {
            from,
            to,
            grain,
            shift,
        } = node.operation()
        else {
            return Err(error(InvariantViolation::NodeOutsideSlice, node));
        };
        self.period_starts(node, (*from, *to), *grain, shift.as_ref())
    }

    /// The period starts from `generate_series`, whose last value is the start of the period
    /// of `to` since it never passes `to`. The bounds are query values: bound parameters,
    /// numbered after the time zone the select item reads (E6-4.2, E6-4.11). With a `shift`,
    /// the bounds are the first day of the shifted query-grain period of `from` and the last
    /// day of the shifted period of `to`, computed in SQL from them, the amounts of the shift
    /// written as literals (E4-8.4, E6-4.14).
    fn period_starts(
        &mut self,
        node: &'input PlanNode,
        (from, to): (DateValue, DateValue),
        grain: Grain,
        shift: Option<&PeriodShift>,
    ) -> Result<SelectBody, GenerateError> {
        self.parameters.start_node(node.origin());
        let column = node
            .schema()
            .columns()
            .first()
            .ok_or_else(|| error(InvariantViolation::UnknownColumn, node))?;
        let start = format!("CAST({START} AS DATE)");
        let value = self.start_sql(node, &start, &column.logical_type)?;
        let (from, to) = match shift {
            Some(shift) => {
                let date = |value: DateValue| Expression::Literal(Literal::Date(value));
                let mut writer = self
                    .writer(node, Columns::Input(node.schema()))
                    .binding_dates_only();
                let from = writer.write(&shift.shifted_period_of(date(from)))?;
                let to = writer.write(&shift.shifted_last_day_of(date(to)))?;
                (from, to)
            }
            None => (self.date_parameter(from), self.date_parameter(to)),
        };
        let first = format!(
            "CAST(date_trunc('{}', {from}) AS TIMESTAMP)",
            grain_name(grain)
        );
        Ok(SelectBody {
            items: vec![format!("{value} AS {}", quote(column.name.as_str()))],
            from: format!(
                "generate_series({first}, CAST({to} AS TIMESTAMP), {}) AS \"menura_periods\"({START})",
                grain_interval(grain)
            ),
            clauses: Vec::new(),
        })
    }

    /// The typed parameter of the query date `date` (E6-4.2a, E6-4.2c).
    fn date_parameter(&mut self, date: DateValue) -> String {
        let placeholder = self
            .parameters
            .bind(LogicalType::Date, Value::String(date_text(date)));
        format!("CAST({placeholder} AS DATE)")
    }
}

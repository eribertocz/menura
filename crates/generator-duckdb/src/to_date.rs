//! The step of a `ToDate` node (E4-3.15, E4-8.5, E6-4.14): each row repeated once for each
//! query-grain period from the period of its time value onwards, within the same `grain`
//! period.
//!
//! The day of the time value (E6-5.2) is truncated to the query grain: that is the start of the
//! row's period p. A `generate_series` from p to p plus one period of the modifier's `grain`,
//! by steps of the query grain, gives every candidate period t; the `WHERE` clause keeps those
//! in the same `grain` period as p (the same ISO week-numbering year for weeks and `year`), and,
//! with a `date_range`, those that overlap it, its bounds being bound parameters (E6-4.11). A
//! row whose time value is NULL is removed. The period column holds the first instant of t, of
//! the type of the time value. The SQL shape is fixed by golden files only (E6-4.14).

use menura_contracts::types::common::Grain;
use menura_representations::logical_plan::{Operation, PlanNode, ToDateGrains, ToDateRange};
use menura_representations::names::ColumnName;

use crate::body::SelectBody;
use crate::day::{grain_interval, grain_name};
use crate::error::{GenerateError, InvariantViolation};
use crate::identifiers::quote;
use crate::steps::{StepWriter, error};

/// The alias of the input rows.
const ROWS: &str = "\"menura_rows\"";

/// The candidate period starts, a `TIMESTAMP` column of the series (E6-4.12).
const START: &str = "\"menura_series\".\"menura.start\"";

impl<'input> StepWriter<'input> {
    /// The body of `node`, a `ToDate`; any other node is not one this function generates.
    pub(crate) fn repeat_to_date(
        &mut self,
        node: &'input PlanNode,
    ) -> Result<SelectBody, GenerateError> {
        let Operation::ToDate {
            input,
            dimension,
            grains,
            range,
            period,
        } = node.operation()
        else {
            return Err(error(InvariantViolation::NodeOutsideSlice, node));
        };
        let written = self.write(input)?;
        self.parameters.start_node(node.origin());
        let time = input
            .schema()
            .columns()
            .iter()
            .find(|column| &column.name == dimension)
            .ok_or_else(|| error(InvariantViolation::UnknownColumn, node))?;
        let value = format!("{ROWS}.{}", quote(dimension.as_str()));
        let day = self.day_sql(node, input, &value, &time.logical_type)?;
        let first = format!(
            "CAST(date_trunc('{}', {day}) AS TIMESTAMP)",
            grain_name(grains.query)
        );
        let mut items = input_items(node, input, period)?;
        let start = self.start_sql(node, &format!("CAST({START} AS DATE)"), &time.logical_type)?;
        items.push(format!("{start} AS {}", quote(period.as_str())));
        let mut conditions = vec![format!("{value} IS NOT NULL"), same_period(*grains, &first)];
        if let Some(range) = range {
            conditions.push(self.overlaps(*range, grains.query));
        }
        Ok(SelectBody {
            items,
            from: format!("{} AS {ROWS}", written.name),
            clauses: vec![
                format!(
                    "CROSS JOIN generate_series({first}, {first} + {}, {}) AS \"menura_series\"(\"menura.start\")",
                    grain_interval(grains.to_date),
                    grain_interval(grains.query)
                ),
                format!("WHERE {}", conditions.join(" AND ")),
            ],
        })
    }

    /// Whether the candidate period, of grain `query`, overlaps `range`: it starts on or before
    /// its last day and ends after its first day.
    fn overlaps(&mut self, range: ToDateRange, query: Grain) -> String {
        let from = self.date_parameter(range.from);
        let to = self.date_parameter(range.to);
        format!(
            "CAST({START} AS DATE) <= {to} AND CAST({START} + {} AS DATE) > {from}",
            grain_interval(query)
        )
    }
}

/// Each column of `input`, read from the input rows, in the order of the schema of `node`,
/// which ends with `period`.
fn input_items(
    node: &PlanNode,
    input: &PlanNode,
    period: &ColumnName,
) -> Result<Vec<String>, GenerateError> {
    let columns = node.schema().columns();
    let (last, passed) = columns
        .split_last()
        .ok_or_else(|| error(InvariantViolation::UnknownColumn, node))?;
    if &last.name != period || passed.len() != input.schema().columns().len() {
        return Err(error(InvariantViolation::UnknownColumn, node));
    }
    passed
        .iter()
        .zip(input.schema().columns())
        .map(|(column, read)| {
            if column.name == read.name {
                let name = quote(column.name.as_str());
                Ok(format!("{ROWS}.{name} AS {name}"))
            } else {
                Err(error(InvariantViolation::UnknownColumn, node))
            }
        })
        .collect()
}

/// Whether the candidate period lies in the same `to_date` period as the row's period, which
/// starts at `first`: the same ISO week-numbering year for week periods and `year`, else the
/// same truncation to `to_date`.
fn same_period(grains: ToDateGrains, first: &str) -> String {
    if grains.query == Grain::Week && grains.to_date == Grain::Year {
        format!("isoyear({START}) = isoyear({first})")
    } else {
        let grain = grain_name(grains.to_date);
        format!("date_trunc('{grain}', {START}) = date_trunc('{grain}', {first})")
    }
}

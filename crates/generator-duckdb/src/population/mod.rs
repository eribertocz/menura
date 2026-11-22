//! The step of a `Population` node (E4-3.12, E6-4.14): the expected (entity, period) pairs of
//! a population at a grain over the query's periods (POP-3).

pub(crate) mod membership;

use menura_contracts::types::common::Grain;
use menura_representations::logical_plan::{Operation, PlanNode, PopulationRule};
use menura_representations::names::ColumnName;

use crate::body::SelectBody;
use crate::error::{GenerateError, InvariantViolation};
use crate::expression::Columns;
use crate::identifiers::quote;
use crate::steps::{StepWriter, error, pass_through};

/// The alias of a row of the population's input.
pub(crate) const ROW: &str = "\"menura_row\"";
/// The alias of a row of the periods.
pub(crate) const PERIOD: &str = "\"menura_period\"";

/// The parts of a `Population` node the step reads.
struct PopulationInputs<'input> {
    input: &'input PlanNode,
    periods: &'input PlanNode,
    period: &'input ColumnName,
    grain: Grain,
    rule: &'input PopulationRule,
}

impl<'input> StepWriter<'input> {
    /// One row per (entity, period) pair where the entity is expected (POP-3): the rows of the
    /// input, kept where `required_when` holds (POP-3.3), each paired with the periods its
    /// membership covers. Several rows of one entity give one pair (POP-3.4); a row with a
    /// NULL entity field gives none (POP-3.7).
    pub(crate) fn population(
        &mut self,
        node: &'input PlanNode,
    ) -> Result<SelectBody, GenerateError> {
        self.population_and_periods(node).map(|(body, _)| body)
    }

    /// Writes the steps of `node`, a `Population`, and returns its step name and the step name
    /// of its periods, which a `Snapshot` that reads it needs (E4-7.7).
    pub(crate) fn write_population(
        &mut self,
        node: &'input PlanNode,
    ) -> Result<(String, String), GenerateError> {
        let (body, periods) = self.population_and_periods(node)?;
        Ok((self.push_step("population", body), periods))
    }

    /// The body of `node`, a `Population`, and the step name of its periods.
    fn population_and_periods(
        &mut self,
        node: &'input PlanNode,
    ) -> Result<(SelectBody, String), GenerateError> {
        let inputs = population_inputs(node)?;
        check_columns(node, &inputs)?;
        let input = self.write(inputs.input)?;
        let periods = self.write(inputs.periods)?;
        self.parameters.start_node(node.origin());
        let rows = match &inputs.rule.required_when {
            Some(condition) => {
                let condition =
                    self.expression(node, Columns::Input(inputs.input.schema()), condition)?;
                let required = SelectBody {
                    items: pass_through(inputs.input, inputs.input.schema())?,
                    from: input.name,
                    clauses: vec![format!("WHERE {condition}")],
                };
                self.push_step("required", required)
            }
            None => input.name,
        };
        let period = format!("{PERIOD}.{}", quote(inputs.period.as_str()));
        let period_type = membership::column_type(node, inputs.periods, inputs.period)?;
        let start = self.day_sql(node, inputs.periods, &period, period_type)?;
        let covered = self.covers(
            node,
            inputs.input,
            &inputs.rule.membership,
            &start,
            inputs.grain,
        )?;
        let body = SelectBody {
            items: output_items(node, inputs.rule, &period)?,
            from: format!("{rows} AS {ROW}"),
            clauses: vec![
                format!("JOIN {} AS {PERIOD} ON {covered}", periods.name),
                format!("WHERE {}", known_entity(inputs.rule)),
                format!("GROUP BY {}", ordinals(node.schema().columns().len())),
            ],
        };
        Ok((body, periods.name))
    }
}

/// The parts of `node`, a `Population`.
fn population_inputs(node: &PlanNode) -> Result<PopulationInputs<'_>, GenerateError> {
    match node.operation() {
        Operation::Population {
            input,
            periods,
            period,
            grain,
            rule,
            ..
        } => Ok(PopulationInputs {
            input,
            periods,
            period,
            grain: *grain,
            rule,
        }),
        _ => Err(error(InvariantViolation::NodeOutsideSlice, node)),
    }
}

/// Checks that the entity columns are columns of the input and that the period column is a
/// column of the periods.
fn check_columns(node: &PlanNode, inputs: &PopulationInputs<'_>) -> Result<(), GenerateError> {
    for name in &inputs.rule.entity {
        membership::column_type(node, inputs.input, name)?;
    }
    membership::column_type(node, inputs.periods, inputs.period).map(|_| ())
}

/// The entity columns of the row, then the period, each named as the node's schema names it.
fn output_items(
    node: &PlanNode,
    rule: &PopulationRule,
    period: &str,
) -> Result<Vec<String>, GenerateError> {
    let names = node.schema().columns();
    if names.len() != rule.entity.len() + 1 {
        return Err(error(InvariantViolation::UnknownColumn, node));
    }
    let values = rule
        .entity
        .iter()
        .map(|name| format!("{ROW}.{}", quote(name.as_str())))
        .chain([period.to_owned()]);
    Ok(values
        .zip(names)
        .map(|(value, column)| format!("{value} AS {}", quote(column.name.as_str())))
        .collect())
}

/// `row.e1 IS NOT NULL AND …`: a row without its whole entity is never expected (POP-3.7).
fn known_entity(rule: &PopulationRule) -> String {
    rule.entity
        .iter()
        .map(|name| format!("{ROW}.{} IS NOT NULL", quote(name.as_str())))
        .collect::<Vec<_>>()
        .join(" AND ")
}

/// `1, 2, …, count`.
fn ordinals(count: usize) -> String {
    (1..=count)
        .map(|ordinal| ordinal.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

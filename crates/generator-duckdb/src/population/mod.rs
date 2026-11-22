//! The step of a `Population` node (E4-3.12, E6-4.14): the expected (entity, period) pairs of
//! a population at a grain over the query's periods (POP-3).

mod membership;

use menura_contracts::types::common::{Grain, LogicalType};
use menura_representations::logical_plan::{Operation, PlanNode, PopulationRule};
use menura_representations::names::ColumnName;

use crate::body::SelectBody;
use crate::error::{GenerateError, InvariantViolation};
use crate::expression::Columns;
use crate::identifiers::quote;
use crate::steps::{StepWriter, error, pass_through};

/// The alias of a row of the population's input.
const ROW: &str = "\"menura_row\"";
/// The alias of a row of the periods.
const PERIOD: &str = "\"menura_period\"";

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
    /// membership covers. Several rows of one entity give one pair (POP-3.4).
    pub(crate) fn population(
        &mut self,
        node: &'input PlanNode,
    ) -> Result<SelectBody, GenerateError> {
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
        let covered = membership::covers(
            node,
            inputs.input,
            &inputs.rule.membership,
            &period,
            inputs.grain,
        )?;
        Ok(SelectBody {
            items: output_items(node, inputs.rule, &period)?,
            from: format!("{rows} AS {ROW}"),
            clauses: vec![
                format!("JOIN {} AS {PERIOD} ON {covered}", periods.name),
                format!("GROUP BY {}", ordinals(node.schema().columns().len())),
            ],
        })
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
/// `date` column of the periods.
fn check_columns(node: &PlanNode, inputs: &PopulationInputs<'_>) -> Result<(), GenerateError> {
    for name in &inputs.rule.entity {
        membership::column_type(node, inputs.input, name)?;
    }
    match membership::column_type(node, inputs.periods, inputs.period)? {
        LogicalType::Date => Ok(()),
        _ => Err(error(InvariantViolation::UnsupportedType, node)),
    }
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

/// `1, 2, …, count`.
fn ordinals(count: usize) -> String {
    (1..=count)
        .map(|ordinal| ordinal.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

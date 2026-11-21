//! The `Project` nodes below the final one that compute the inputs of derived metrics, one
//! level of derivation per `Project`, so that each formula is written once, with the division
//! rule of its own derived metric (DER-3.1, E6-4.6a).

use menura_representations::logical_plan::{
    NamedExpression, Operation, OutputSchema, PlanNode, SchemaColumn,
};
use menura_representations::origin::Origin;

use crate::columns::column_reference;
use crate::error::{InvariantViolation, PlanError};
use crate::expansion::InputColumn;

/// `input` under one `Project` per level of `inputs`, from level 0 up: each passes every
/// column of its input through, in order, then computes the input columns of its level, in
/// the order of `inputs`. `input` unchanged without input columns.
///
/// # Errors
///
/// An internal error for a column of `input` with an empty name.
pub(crate) fn compute_inputs(
    input: PlanNode,
    inputs: &[&InputColumn],
    origin: &Origin,
) -> Result<PlanNode, PlanError> {
    let Some(highest) = inputs.iter().map(|column| column.level).max() else {
        return Ok(input);
    };
    let mut node = input;
    for level in 0..=highest {
        let at_level: Vec<&InputColumn> = inputs
            .iter()
            .copied()
            .filter(|column| column.level == level)
            .collect();
        node = level_project(node, &at_level, origin)
            .map_err(|violation| PlanError::invariant(violation, origin))?;
    }
    Ok(node)
}

/// The `Project` of `input` that passes its columns through and computes `columns`.
fn level_project(
    input: PlanNode,
    columns: &[&InputColumn],
    origin: &Origin,
) -> Result<PlanNode, InvariantViolation> {
    let mut expressions = Vec::new();
    let mut schema = Vec::new();
    for column in input.schema().columns() {
        expressions.push(NamedExpression {
            name: column.name.clone(),
            expression: column_reference(column.name.as_str())?,
            derived: None,
        });
        schema.push(column.clone());
    }
    for column in columns {
        expressions.push(NamedExpression {
            name: column.name.clone(),
            expression: column.expression.clone(),
            derived: column.derived.clone(),
        });
        schema.push(SchemaColumn {
            name: column.name.clone(),
            logical_type: column.logical_type.clone(),
            nullable: column.nullable,
        });
    }
    Ok(PlanNode::new(
        Operation::Project {
            input: Box::new(input),
            expressions,
        },
        OutputSchema::new(schema),
        origin.clone(),
    ))
}

//! The `Lookup` of a context query (E4-3.16, E4-8.7): the components of its base attached to
//! each result row.

use menura_model::expression::Literal;
use menura_representations::logical_plan::{
    LookupColumn, LookupKey, Operation, OutputSchema, PlanNode, SchemaColumn,
};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;

use crate::columns::{component_column, context_component_column, dimension_column};
use crate::context::ContextQuery;
use crate::error::{InvariantViolation, PlanError};
use crate::stitch::empty_value;

/// The `Lookup` of `value`, the rows of the context query `context` at `position` (E4-8.8),
/// over the result rows `result`. Its mapping matches each dimension the context query keeps
/// with the query dimension it comes from (E4-8.3). It attaches each component of the base
/// under the column [`context_component_column`] of `position`, with its empty-input value
/// (E4-5.0b). Its output is the columns of `result`, then the attached columns; an attached
/// column can be NULL unless both its value and its empty-input value are non-NULL.
///
/// # Errors
///
/// [`InvariantViolation::UnknownComponent`] when `value` does not end with a `Stitch` or an
/// `Aggregate`, or lacks a key or a component of the base.
pub(crate) fn lookup(
    result: PlanNode,
    value: PlanNode,
    position: usize,
    context: &ContextQuery,
) -> Result<PlanNode, PlanError> {
    let origin = Origin::Model(context.location().clone());
    let fail = || PlanError::invariant(InvariantViolation::UnknownComponent, &origin);
    let mapping = context
        .kept()
        .iter()
        .enumerate()
        .map(|(index, query_position)| {
            let value_key = dimension_column(index);
            let result_key = dimension_column(*query_position);
            let both = has_column(&value, &value_key) && has_column(&result, &result_key);
            both.then_some(LookupKey::Equal {
                result: result_key,
                value: value_key,
            })
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(fail)?;
    let mut schema: Vec<SchemaColumn> = result.schema().columns().to_vec();
    let mut columns = Vec::new();
    for (component, empty) in component_values(&value).ok_or_else(fail)? {
        let value_column = component_column(0, component);
        let found = schema_column(&value, &value_column).ok_or_else(fail)?;
        let name = context_component_column(position, component);
        schema.push(SchemaColumn {
            name: name.clone(),
            logical_type: found.logical_type.clone(),
            nullable: found.nullable || empty.is_none(),
        });
        columns.push(LookupColumn {
            value: value_column,
            name,
            empty_value: empty,
        });
    }
    Ok(PlanNode::new(
        Operation::Lookup {
            result: Box::new(result),
            value: Box::new(value),
            mapping,
            columns,
        },
        OutputSchema::new(schema),
        origin,
    ))
}

/// The position of each component of the base that `value` computes, with its empty-input
/// value, in component order: the context query's only metric is the base, at position 0.
/// `None` when `value` does not end with a `Stitch` or an `Aggregate`.
fn component_values(value: &PlanNode) -> Option<Vec<(usize, Option<Literal>)>> {
    let mut found: Vec<(usize, Option<Literal>)> = match value.operation() {
        Operation::Stitch { columns, .. } => columns
            .iter()
            .map(|column| {
                Some((
                    component_position(&column.name)?,
                    column.empty_value.clone(),
                ))
            })
            .collect::<Option<_>>()?,
        Operation::Aggregate { components, .. } => components
            .iter()
            .map(|component| {
                Some((
                    component_position(&component.name)?,
                    empty_value(&component.call).ok()?,
                ))
            })
            .collect::<Option<_>>()?,
        _ => return None,
    };
    found.sort_by_key(|(position, _)| *position);
    Some(found)
}

/// The position of the component of the metric at position 0 whose column is `name`.
fn component_position(name: &ColumnName) -> Option<usize> {
    let number: usize = name
        .as_str()
        .strip_prefix("metric_1_component_")?
        .parse()
        .ok()?;
    number.checked_sub(1)
}

fn has_column(node: &PlanNode, name: &ColumnName) -> bool {
    schema_column(node, name).is_some()
}

fn schema_column<'plan>(node: &'plan PlanNode, name: &ColumnName) -> Option<&'plan SchemaColumn> {
    node.schema()
        .columns()
        .iter()
        .find(|column| &column.name == name)
}

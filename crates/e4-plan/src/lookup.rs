//! The `Lookup` of a context query (E4-3.16, E4-8.7): the components of its base attached to
//! each result row.

use menura_contracts::types::common::LogicalType;
use menura_model::expression::{Expression, Literal};
use menura_representations::logical_plan::{
    LookupColumn, LookupKey, NamedExpression, Operation, OutputSchema, PlanNode, SchemaColumn,
};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;

use crate::columns::{
    column_reference, component_column, context_component_column, dimension_column,
};
use crate::context::{ContextQuery, ShiftedTime};
use crate::error::{InvariantViolation, PlanError};
use crate::stitch::empty_value;

/// The `Lookup` of `value`, the rows of the context query `context` at `position` (E4-8.8),
/// over the result rows `result`. Its mapping matches each dimension the context query keeps
/// with the query dimension it comes from (E4-8.3). It attaches each component of the base
/// under the column [`context_component_column`] of `position`, with its empty-input value
/// (E4-5.0b). For `prior_period`, the time key of the result row is shifted first and matched
/// with the time key of the context query (E4-8.4, [`TimeKeys`]). Its output is the columns of `result`, then the attached columns; an attached
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
    let components = component_values(&value).ok_or_else(fail)?;
    let (result, value, time_keys) = match context.shifted_time() {
        Some(time) => {
            let keys = TimeKeys::new(position, time);
            let result = keys.shifted_result(result, &origin).ok_or_else(fail)?;
            let value = keys.present_value(value, &origin).ok_or_else(fail)?;
            (result, value, Some(keys))
        }
        None => (result, value, None),
    };
    let mapping = mapping(context, &result, &value, time_keys.as_ref()).ok_or_else(fail)?;
    let mut schema: Vec<SchemaColumn> = result.schema().columns().to_vec();
    let mut columns = Vec::new();
    for (component, empty) in components {
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

/// The keys of the mapping: each dimension the context query keeps matched with the query
/// dimension it comes from, except a `prior_period` time key, matched with the result row's
/// shifted time key, and then the presence flags of `time_keys` (E4-8.3, E4-8.4). `None` when
/// `result` or `value` lacks a key column.
fn mapping(
    context: &ContextQuery,
    result: &PlanNode,
    value: &PlanNode,
    time_keys: Option<&TimeKeys>,
) -> Option<Vec<LookupKey>> {
    let mut mapping = context
        .kept()
        .iter()
        .enumerate()
        .map(|(index, query_position)| {
            let value_key = dimension_column(index);
            let result_key = match time_keys {
                Some(keys) if keys.time.position == index => keys.shifted.clone(),
                _ => dimension_column(*query_position),
            };
            let both = has_column(value, &value_key) && has_column(result, &result_key);
            both.then_some(LookupKey::Equal {
                result: result_key,
                value: value_key,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    if let Some(keys) = time_keys {
        mapping.push(LookupKey::Equal {
            result: keys.present.clone(),
            value: keys.present.clone(),
        });
    }
    Some(mapping)
}

/// The columns that match the time key of a `prior_period` context query (E4-8.4, E4-8.7):
/// the result row's time key shifted, compared with the context query's time key, and a
/// presence flag, TRUE on every result row and TRUE on a context row whose time key is not
/// NULL, so that a result row whose time key is NULL matches no row and takes the empty-input
/// values, keys being compared with NULLs equal.
struct TimeKeys {
    time: ShiftedTime,
    shifted: ColumnName,
    present: ColumnName,
}

impl TimeKeys {
    fn new(position: usize, time: ShiftedTime) -> Self {
        Self {
            time,
            shifted: ColumnName::new(&format!("context_{}_shifted_time", position + 1)),
            present: ColumnName::new(&format!("context_{}_time_present", position + 1)),
        }
    }

    /// `result` with the shifted time key and the presence flag TRUE (CTX-3a.2). `None` when
    /// `result` lacks the time key.
    fn shifted_result(&self, result: PlanNode, origin: &Origin) -> Option<PlanNode> {
        let key = dimension_column(self.time.position);
        let key_type = schema_column(&result, &key)?.logical_type.clone();
        let shifted = self
            .time
            .shift
            .shifted(column_reference(key.as_str()).ok()?);
        let added = [
            (self.shifted.clone(), shifted, key_type, true),
            (
                self.present.clone(),
                Expression::Literal(Literal::Boolean(true)),
                LogicalType::Boolean,
                false,
            ),
        ];
        extended(result, added, origin)
    }

    /// `value` with the presence flag: whether its time key is not NULL. `None` when `value`
    /// lacks the time key.
    fn present_value(&self, value: PlanNode, origin: &Origin) -> Option<PlanNode> {
        let key = dimension_column(self.time.position);
        let present = Expression::IsNull {
            operand: Box::new(column_reference(key.as_str()).ok()?),
            negated: true,
        };
        has_column(&value, &key).then_some(())?;
        let added = [(self.present.clone(), present, LogicalType::Boolean, false)];
        extended(value, added, origin)
    }
}

/// A `Project` of every column of `input`, then the `added` columns, each with its expression,
/// type and nullability.
fn extended<const N: usize>(
    input: PlanNode,
    added: [(ColumnName, Expression, LogicalType, bool); N],
    origin: &Origin,
) -> Option<PlanNode> {
    let mut schema: Vec<SchemaColumn> = input.schema().columns().to_vec();
    let mut expressions = schema
        .iter()
        .map(|column| {
            Some(NamedExpression {
                name: column.name.clone(),
                expression: column_reference(column.name.as_str()).ok()?,
                derived: None,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    for (name, expression, logical_type, nullable) in added {
        schema.push(SchemaColumn {
            name: name.clone(),
            logical_type,
            nullable,
        });
        expressions.push(NamedExpression {
            name,
            expression,
            derived: None,
        });
    }
    Some(PlanNode::new(
        Operation::Project {
            input: Box::new(input),
            expressions,
        },
        OutputSchema::new(schema),
        origin.clone(),
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

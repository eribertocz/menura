//! The output of the plan: the `Project` of the output columns, then `Sort` and `Limit`
//! (E4-3, E4-9).

use menura_representations::bound_query::{BoundDimension, BoundQuery};
use menura_representations::columns::OutputColumn;
use menura_representations::logical_plan::{
    NamedExpression, Operation, OutputSchema, PlanNode, SchemaColumn, SortItem,
};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;

use crate::columns::{column_reference, dimension_column};
use crate::dimensions::at;
use crate::error::{InvariantViolation, PlanError};
use crate::metrics::MetricOutput;
use crate::typing::typed_column;

/// The `Project` of the output columns of `query`, in output order (E1-5), named with their
/// output names (E1-5.1): each dimension reads its Aggregate key, each metric is evaluated
/// from its components.
///
/// # Errors
///
/// An internal error when a dimension has no key in `aggregate`.
pub(crate) fn project(
    query: &BoundQuery,
    aggregate: PlanNode,
    metrics: Vec<MetricOutput>,
    origin: Origin,
) -> Result<PlanNode, PlanError> {
    let mut metrics = metrics.into_iter();
    let mut dimension_position = 0;
    let mut expressions = Vec::new();
    let mut columns = Vec::new();
    for column in query.columns().iter() {
        match column {
            OutputColumn::Dimension(dimension) => {
                let (expression, column) =
                    dimension_output(dimension, dimension_position, &aggregate)?;
                dimension_position += 1;
                expressions.push(expression);
                columns.push(column);
            }
            OutputColumn::Metric(_) => {
                let Some(metric) = metrics.next() else {
                    return Err(PlanError::invariant(
                        InvariantViolation::UnknownMetric,
                        &origin,
                    ));
                };
                columns.push(metric_column(&metric, &aggregate, &origin)?);
                expressions.push(NamedExpression {
                    name: metric.name,
                    expression: metric.expression,
                    derived: metric.derived,
                });
            }
        }
    }
    Ok(PlanNode::new(
        Operation::Project {
            input: Box::new(aggregate),
            expressions,
        },
        OutputSchema::new(columns),
        origin,
    ))
}

/// The output column of `metric`: with the type E0.f gave its metric, or for an inline metric
/// the type of its expression over the columns of `input`.
fn metric_column(
    metric: &MetricOutput,
    input: &PlanNode,
    origin: &Origin,
) -> Result<SchemaColumn, PlanError> {
    match &metric.result_type {
        Some(result_type) => Ok(SchemaColumn {
            name: metric.name.clone(),
            logical_type: result_type.logical_type.clone(),
            nullable: result_type.nullable,
        }),
        None => typed_column(metric.name.clone(), &metric.expression, input.schema())
            .map_err(|violation| PlanError::invariant(violation, origin)),
    }
}

/// The output column of the dimension at `position` among the dimensions: it reads the
/// Aggregate key of that dimension, with the key's type.
fn dimension_output(
    dimension: &BoundDimension,
    position: usize,
    aggregate: &PlanNode,
) -> Result<(NamedExpression, SchemaColumn), PlanError> {
    let key = dimension_column(position);
    let fail = |violation| at(violation, &dimension.origin);
    let typed = aggregate
        .schema()
        .columns()
        .iter()
        .find(|candidate| candidate.name == key)
        .ok_or_else(|| fail(InvariantViolation::UntypedExpression))?;
    let name = ColumnName::new(dimension.output_name.as_str());
    let expression = NamedExpression {
        name: name.clone(),
        expression: column_reference(key.as_str()).map_err(fail)?,
        derived: None,
    };
    let column = SchemaColumn {
        name,
        ..typed.clone()
    };
    Ok((expression, column))
}

/// `input` sorted by the explicit ordering terms of `query` only (E4-9.1), unchanged without
/// `order_by` (E4-9.2), then limited.
///
/// # Errors
///
/// An internal error for an ordering term on a column `input` does not output.
pub(crate) fn order_and_limit(query: &BoundQuery, input: PlanNode) -> Result<PlanNode, PlanError> {
    let mut node = input;
    if let Some(first) = query.order().first() {
        let items = query
            .order()
            .iter()
            .map(|item| {
                let column = node
                    .schema()
                    .columns()
                    .get(item.column)
                    .ok_or_else(|| at(InvariantViolation::UnknownOrderColumn, &item.origin))?;
                Ok(SortItem {
                    column: column.name.clone(),
                    direction: item.direction,
                    nulls: item.nulls,
                })
            })
            .collect::<Result<Vec<_>, PlanError>>()?;
        let schema = node.schema().clone();
        node = PlanNode::new(
            Operation::Sort {
                input: Box::new(node),
                items,
            },
            schema,
            Origin::Query(first.origin.clone()),
        );
    }
    if let Some(limit) = query.limit() {
        let schema = node.schema().clone();
        node = PlanNode::new(
            Operation::Limit {
                input: Box::new(node),
                rows: limit.rows,
            },
            schema,
            Origin::Query(limit.origin.clone()),
        );
    }
    Ok(node)
}

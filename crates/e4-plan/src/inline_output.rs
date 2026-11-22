//! The output of a query with inline metrics of the SQL form (E1-4.2): the formula of an inline
//! metric is written in the query, so it is computed in a `Project` whose origin is the query,
//! where its literals are bound parameters (E6-4.2), above the `Project` of the model metrics,
//! whose origin is the model, where model constants stay literals (E6-4.11).

use menura_representations::bound_query::{BoundMetricKind, BoundQuery};
use menura_representations::columns::OutputColumn;
use menura_representations::logical_plan::{
    NamedExpression, Operation, OutputSchema, PlanNode, SchemaColumn,
};
use menura_representations::names::ColumnName;
use menura_representations::origin::{Origin, QueryLocation};

use crate::columns::{column_reference, dimension_column, metric_value_column};
use crate::error::{InvariantViolation, PlanError};
use crate::metrics::MetricOutput;
use crate::typing::typed_column;

/// Where the first inline metric of `query` is written; `None` when it has none.
pub(crate) fn first_inline_metric(query: &BoundQuery) -> Option<&QueryLocation> {
    query
        .columns()
        .metrics()
        .find(|metric| matches!(metric.kind, BoundMetricKind::Inline(_)))
        .map(|metric| &metric.origin)
}

/// The output of `query` over `input`: a `Project` from `origin` that passes the columns of
/// `input` through and computes each model metric of `metrics` under its
/// [`metric_value_column`], then a `Project` from `inline_origin` of the output columns, in
/// output order, named with their output names: each dimension reads its Aggregate key, each
/// model metric its value, and each inline metric is its formula over the component and input
/// columns.
///
/// # Errors
///
/// An internal error when `metrics` does not match the metrics of `query`, or an output
/// column cannot be typed.
pub(crate) fn project(
    query: &BoundQuery,
    input: PlanNode,
    metrics: Vec<MetricOutput>,
    origin: &Origin,
    inline_origin: Origin,
) -> Result<PlanNode, PlanError> {
    let fail = |violation| PlanError::invariant(violation, origin);
    let mut values = pass_through(&input).map_err(fail)?;
    let mut outputs = Vec::new();
    let mut metrics = metrics.into_iter().enumerate();
    let mut dimension_position = 0;
    for column in query.columns().iter() {
        let output = match column {
            OutputColumn::Dimension(dimension) => {
                let key = dimension_column(dimension_position);
                dimension_position += 1;
                read(dimension.output_name.as_str(), &key).map_err(fail)?
            }
            OutputColumn::Metric(_) => {
                let (position, metric) = metrics
                    .next()
                    .ok_or_else(|| fail(InvariantViolation::UnknownMetric))?;
                output_of_metric(position, metric, &mut values).map_err(fail)?
            }
        };
        outputs.push(output);
    }
    let lower = values.into_node(input, origin.clone());
    let columns = outputs
        .iter()
        .map(|output| typed_column(output.name.clone(), &output.expression, lower.schema()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|violation| PlanError::invariant(violation, &inline_origin))?;
    Ok(project_node(lower, outputs, columns, inline_origin))
}

/// The expressions and schema of the `Project` of the model metrics.
struct Values {
    expressions: Vec<NamedExpression>,
    columns: Vec<SchemaColumn>,
}

impl Values {
    fn into_node(self, input: PlanNode, origin: Origin) -> PlanNode {
        project_node(input, self.expressions, self.columns, origin)
    }
}

/// The values that pass every column of `input` through, in order.
fn pass_through(input: &PlanNode) -> Result<Values, InvariantViolation> {
    let mut values = Values {
        expressions: Vec::new(),
        columns: Vec::new(),
    };
    for column in input.schema().columns() {
        values.expressions.push(NamedExpression {
            name: column.name.clone(),
            expression: column_reference(column.name.as_str())?,
            derived: None,
        });
        values.columns.push(column.clone());
    }
    Ok(values)
}

/// The output column of the metric at `position`: an inline metric is its formula; a model
/// metric is computed in `values` and read from there.
fn output_of_metric(
    position: usize,
    metric: MetricOutput,
    values: &mut Values,
) -> Result<NamedExpression, InvariantViolation> {
    let Some(result_type) = metric.result_type else {
        return Ok(NamedExpression {
            name: metric.name,
            expression: metric.expression,
            derived: None,
        });
    };
    let value = metric_value_column(position);
    values.columns.push(SchemaColumn {
        name: value.clone(),
        logical_type: result_type.logical_type,
        nullable: result_type.nullable,
    });
    values.expressions.push(NamedExpression {
        name: value.clone(),
        expression: metric.expression,
        derived: metric.derived,
    });
    read(metric.name.as_str(), &value)
}

/// The output column `name` that reads the column `column`.
fn read(name: &str, column: &ColumnName) -> Result<NamedExpression, InvariantViolation> {
    Ok(NamedExpression {
        name: ColumnName::new(name),
        expression: column_reference(column.as_str())?,
        derived: None,
    })
}

fn project_node(
    input: PlanNode,
    expressions: Vec<NamedExpression>,
    columns: Vec<SchemaColumn>,
    origin: Origin,
) -> PlanNode {
    PlanNode::new(
        Operation::Project {
            input: Box::new(input),
            expressions,
        },
        OutputSchema::new(columns),
        origin,
    )
}

//! `EQRY-03` for the dimensions and metrics whose model expression reads the current date or
//! time, directly or through the fields and metrics it reads (QRY-2, E2-5.6, E0F-2.3).

use std::collections::BTreeSet;

use menura_model::expression::{Expression, Reference};
use menura_model::internal_model::{FieldPath, InternalModel, Metric, NormalizedName};
use menura_representations::bound_query::{
    BoundDimension, BoundDimensionKind, BoundMetric, BoundMetricKind, MetricPath,
};
use menura_representations::columns::{Columns, OutputColumn};
use menura_representations::origin::QueryLocation;

use crate::model_elements::metric;
use crate::names::identifier_name;
use crate::walk::nodes;

/// Where the first output column in query order whose model expression reads the current
/// date or time is written; `None` when none does.
pub(crate) fn first_column_reading_now<'query>(
    model: &InternalModel,
    columns: &'query Columns<BoundDimension, BoundMetric>,
) -> Option<&'query QueryLocation> {
    columns.iter().find_map(|column| match column {
        OutputColumn::Dimension(dimension) => {
            dimension_reads_now(model, dimension).then_some(&dimension.origin)
        }
        OutputColumn::Metric(bound) => {
            let mut visited = BTreeSet::new();
            metric_kind_reads_now(model, &bound.kind, &mut visited).then_some(&bound.origin)
        }
    })
}

/// Whether `expression` itself holds `CURRENT_DATE`, `CURRENT_TIME` or `CURRENT_TIMESTAMP`.
pub(crate) fn holds_now(expression: &Expression) -> bool {
    nodes(expression).into_iter().any(|node| {
        matches!(
            node,
            Expression::CurrentDate | Expression::CurrentTime | Expression::CurrentTimestamp
        )
    })
}

/// Whether the dimension reads the current date or time through its field, or, in the SQL
/// form, directly or through the fields of its dataset.
fn dimension_reads_now(model: &InternalModel, dimension: &BoundDimension) -> bool {
    match &dimension.kind {
        BoundDimensionKind::Field { field, .. } => field_reads_now(model, field),
        BoundDimensionKind::Expression {
            dataset,
            expression,
        } => {
            holds_now(expression)
                || references(expression).any(|reference| {
                    field_path(reference, Some(dataset))
                        .is_some_and(|path| field_reads_now(model, &path))
                })
        }
    }
}

/// Whether the field `path` reads the current date or time. A field expression reads source
/// columns only, never other fields (E0D-3).
fn field_reads_now(model: &InternalModel, path: &FieldPath) -> bool {
    model
        .dataset(&path.dataset)
        .and_then(|dataset| dataset.field(&path.field))
        .and_then(|field| field.expression.as_ref())
        .is_some_and(holds_now)
}

/// Whether a bound metric reads the current date or time.
fn metric_kind_reads_now(
    model: &InternalModel,
    kind: &BoundMetricKind,
    visited: &mut BTreeSet<MetricPath>,
) -> bool {
    match kind {
        BoundMetricKind::Reference(path) => metric_reads_now(model, path, visited),
        BoundMetricKind::Inline(expression) => {
            expression_reads_now(model, expression, None, visited)
        }
    }
}

/// Whether the metric `path` reads the current date or time, directly or through the
/// fields and metrics it reads. A metric already in `visited` is not read again, so a cycle
/// ends.
fn metric_reads_now(
    model: &InternalModel,
    path: &MetricPath,
    visited: &mut BTreeSet<MetricPath>,
) -> bool {
    if !visited.insert(path.clone()) {
        return false;
    }
    let Some(found) = metric(model, path) else {
        return false;
    };
    let scope = match path {
        MetricPath::Dataset { dataset, .. } => Some(dataset),
        MetricPath::SemanticModel(_) => None,
    };
    let reads_itself = found
        .expression
        .as_ref()
        .is_some_and(|expression| expression_reads_now(model, expression, scope, visited));
    reads_itself
        || input_metrics(model, found, scope)
            .iter()
            .any(|input| metric_reads_now(model, input, visited))
}

/// Whether a metric expression of the dataset `scope` (`None` for the semantic model) reads
/// the current date or time: itself, through a field it reads, or through a metric it reads
/// with `AGG(…)`. A `source.column` reference of a dataset metric reads a source column, not
/// a field (E0D-3).
fn expression_reads_now(
    model: &InternalModel,
    expression: &Expression,
    scope: Option<&NormalizedName>,
    visited: &mut BTreeSet<MetricPath>,
) -> bool {
    nodes(expression).into_iter().any(|node| match node {
        Expression::CurrentDate | Expression::CurrentTime | Expression::CurrentTimestamp => true,
        Expression::Reference(reference) if !reads_source_column(reference, scope) => {
            field_path(reference, scope).is_some_and(|path| field_reads_now(model, &path))
        }
        Expression::MetricAggregate(reference) => {
            metric_reads_now(model, &input_path(reference), visited)
        }
        _ => false,
    })
}

/// The metrics a metric reads through its extensions: the inputs of a derived metric and the
/// base of a context metric, which is a metric of its own dataset when that dataset has one of
/// that name, otherwise a metric of the semantic model.
fn input_metrics(
    model: &InternalModel,
    found: &Metric,
    scope: Option<&NormalizedName>,
) -> Vec<MetricPath> {
    let mut inputs: Vec<MetricPath> = found
        .derived
        .iter()
        .flat_map(|derived| derived.inputs.iter().cloned())
        .map(MetricPath::SemanticModel)
        .collect();
    if let Some(context) = &found.context {
        let own = scope.map(|dataset| MetricPath::Dataset {
            dataset: dataset.clone(),
            metric: context.base.clone(),
        });
        inputs.push(match own {
            Some(path) if metric(model, &path).is_some() => path,
            _ => MetricPath::SemanticModel(context.base.clone()),
        });
    }
    inputs
}

/// Whether `reference` is `source.column` in a dataset metric.
fn reads_source_column(reference: &Reference, scope: Option<&NormalizedName>) -> bool {
    scope.is_some()
        && reference
            .qualifier
            .as_ref()
            .is_some_and(|qualifier| identifier_name(qualifier).as_str() == "SOURCE")
}

/// The field a metric expression reference names: `dataset.field`, or a bare field of the
/// dataset `scope`; `None` for a bare name outside a dataset.
fn field_path(reference: &Reference, scope: Option<&NormalizedName>) -> Option<FieldPath> {
    let dataset = reference
        .qualifier
        .as_ref()
        .map(identifier_name)
        .or_else(|| scope.cloned())?;
    Some(FieldPath {
        dataset,
        field: identifier_name(&reference.name),
    })
}

/// The metric `AGG(reference)` reads: `AGG(metric)` a metric of the semantic model,
/// `AGG(dataset.metric)` a dataset metric.
fn input_path(reference: &Reference) -> MetricPath {
    let name = identifier_name(&reference.name);
    match &reference.qualifier {
        Some(dataset) => MetricPath::Dataset {
            dataset: identifier_name(dataset),
            metric: name,
        },
        None => MetricPath::SemanticModel(name),
    }
}

/// The references of `expression`, in written order.
fn references(expression: &Expression) -> impl Iterator<Item = &Reference> {
    nodes(expression).into_iter().filter_map(|node| match node {
        Expression::Reference(reference) => Some(reference),
        _ => None,
    })
}

//! The metric components of the branch, and each metric evaluated from its components
//! (E4-4 step 6, E4-3 `Project`).

use menura_contracts::types::common::LogicalType;
use menura_model::expression::Expression;
use menura_model::internal_model::{
    Dataset, DeclaredAdditivity, InternalModel, Metric, MetricClassification, SourceLocation,
    ValueType,
};
use menura_representations::bound_query::{BoundMetric, BoundMetricKind, BoundQuery, MetricPath};
use menura_representations::logical_plan::AggregateComponent;
use menura_representations::names::ColumnName;
use menura_representations::paths::{Branch, ComponentKey};

use crate::columns::{column_reference, component_column, over_scan};
use crate::dimensions::at;
use crate::error::{InvariantViolation, PlanError};
use crate::rewrite::rewrite;

/// The model location of the first metric of `metrics`, where the final `Project` comes
/// from; `None` without metrics.
pub(crate) fn first_metric_location(metrics: &[PlannedMetric<'_>]) -> Option<SourceLocation> {
    metrics.first().map(|metric| metric.model.location.clone())
}

/// A metric of the query, read from the model.
pub(crate) struct PlannedMetric<'input> {
    bound: &'input BoundMetric,
    model: &'input Metric,
    classification: &'input MetricClassification,
    in_dataset_metric: bool,
}

/// A metric output column, evaluated from the components.
pub(crate) struct MetricOutput {
    /// Its output name.
    pub(crate) name: ColumnName,
    /// Its expression over the Aggregate columns.
    pub(crate) expression: Expression,
    /// Its result type, as E0.f typed the metric.
    pub(crate) logical_type: LogicalType,
    /// Whether it can be NULL.
    pub(crate) nullable: bool,
}

/// The metrics of `query`, in output order, each read from `model`.
///
/// # Errors
///
/// An internal error for an inline metric, a metric the model does not have or did not
/// classify, or a metric with the derived, additivity, population or context extension.
pub(crate) fn planned_metrics<'input>(
    query: &'input BoundQuery,
    model: &'input InternalModel,
) -> Result<Vec<PlannedMetric<'input>>, PlanError> {
    query
        .columns()
        .metrics()
        .map(|bound| planned_metric(bound, model).map_err(|violation| at(violation, &bound.origin)))
        .collect()
}

fn planned_metric<'input>(
    bound: &'input BoundMetric,
    model: &'input InternalModel,
) -> Result<PlannedMetric<'input>, InvariantViolation> {
    let BoundMetricKind::Reference(path) = &bound.kind else {
        return Err(InvariantViolation::InlineMetric);
    };
    let found = model_metric(path, model).ok_or(InvariantViolation::UnknownMetric)?;
    check_extensions(found)?;
    let classification = found
        .classification
        .as_ref()
        .ok_or(InvariantViolation::UnknownMetric)?;
    Ok(PlannedMetric {
        bound,
        model: found,
        classification,
        in_dataset_metric: matches!(path, MetricPath::Dataset { .. }),
    })
}

/// The metric `path` names.
fn model_metric<'model>(path: &MetricPath, model: &'model InternalModel) -> Option<&'model Metric> {
    match path {
        MetricPath::SemanticModel(name) => model.metric(name),
        MetricPath::Dataset { dataset, metric } => model
            .dataset(dataset)?
            .metrics
            .iter()
            .find(|candidate| &candidate.name == metric),
    }
}

/// Fails for a metric extension outside the slice (VS-1).
fn check_extensions(metric: &Metric) -> Result<(), InvariantViolation> {
    if metric.derived.is_some() {
        return Err(InvariantViolation::DerivedMetric);
    }
    if metric.context.is_some() {
        return Err(InvariantViolation::ContextMetric);
    }
    match &metric.declared_additivity {
        Some(DeclaredAdditivity::SemiAdditive(semi)) if semi.population.is_some() => {
            Err(InvariantViolation::PopulationMetric)
        }
        Some(_) => Err(InvariantViolation::AdditivityMetric),
        None => Ok(()),
    }
}

/// The Aggregate component of every key of `branch`, in branch order, over the `Scan`
/// columns of `home`.
///
/// # Errors
///
/// An internal error for a key that names no component, a component of another dataset, or
/// a source column.
pub(crate) fn branch_components(
    branch: &Branch,
    metrics: &[PlannedMetric<'_>],
    home: &Dataset,
) -> Result<Vec<AggregateComponent>, PlanError> {
    branch
        .components
        .iter()
        .map(|key| component(*key, metrics, home))
        .collect()
}

fn component(
    key: ComponentKey,
    metrics: &[PlannedMetric<'_>],
    home: &Dataset,
) -> Result<AggregateComponent, PlanError> {
    let unknown = || {
        PlanError::invariant(
            InvariantViolation::UnknownComponent,
            &crate::slice::query_root(),
        )
    };
    let metric = metrics.get(key.metric).ok_or_else(unknown)?;
    let fail = |violation| at(violation, &metric.bound.origin);
    let found = metric
        .classification
        .components
        .get(key.component)
        .ok_or_else(|| fail(InvariantViolation::UnknownComponent))?;
    if found.home_dataset != home.name {
        return Err(fail(InvariantViolation::ForeignField));
    }
    let aggregate = Expression::Aggregate {
        call: found.call.clone(),
        filter: found.filter.clone().map(Box::new),
    };
    let Expression::Aggregate { call, filter } =
        over_scan(&aggregate, metric.in_dataset_metric).map_err(fail)?
    else {
        return Err(fail(InvariantViolation::ComponentMismatch));
    };
    Ok(AggregateComponent {
        name: component_column(key.metric, key.component),
        call,
        filter: filter.map(|filter| *filter),
    })
}

/// Every metric, in output order, evaluated from its components in `branch`.
///
/// # Errors
///
/// An internal error for a metric whose aggregates do not match its components, a component
/// missing from `branch`, or a metric without a type.
pub(crate) fn metric_outputs(
    branch: &Branch,
    metrics: &[PlannedMetric<'_>],
) -> Result<Vec<MetricOutput>, PlanError> {
    metrics
        .iter()
        .enumerate()
        .map(|(position, metric)| {
            metric_output(branch, position, metric)
                .map_err(|violation| at(violation, &metric.bound.origin))
        })
        .collect()
}

fn metric_output(
    branch: &Branch,
    position: usize,
    metric: &PlannedMetric<'_>,
) -> Result<MetricOutput, InvariantViolation> {
    let expression = metric
        .model
        .expression
        .as_ref()
        .ok_or(InvariantViolation::ComponentMismatch)?;
    let mut next = 0;
    let evaluated = rewrite(expression, &mut |node| {
        let Expression::Aggregate { .. } = node else {
            return Ok(None);
        };
        let key = ComponentKey {
            metric: position,
            component: next,
        };
        next += 1;
        if !branch.components.contains(&key) {
            return Err(InvariantViolation::UnknownComponent);
        }
        column_reference(component_column(key.metric, key.component).as_str()).map(Some)
    })?;
    if next != metric.classification.components.len() {
        return Err(InvariantViolation::ComponentMismatch);
    }
    let typed = metric
        .model
        .typed
        .as_ref()
        .ok_or(InvariantViolation::UntypedExpression)?;
    let ValueType::Logical(logical_type) = &typed.value_type else {
        return Err(InvariantViolation::UntypedExpression);
    };
    Ok(MetricOutput {
        name: ColumnName::new(metric.bound.output_name.as_str()),
        expression: evaluated,
        logical_type: logical_type.clone(),
        nullable: typed.nullable,
    })
}

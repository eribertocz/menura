//! The metric components of the branch, and each metric evaluated from its components
//! (E4-4 step 6, E4-3 `Project`).

use menura_contracts::types::common::LogicalType;
use menura_model::expression::Expression;
use menura_model::internal_model::{
    Dataset, DeclaredAdditivity, InternalModel, Metric, SemiAdditive, SourceLocation, ValueType,
};
use menura_representations::bound_query::{BoundMetric, BoundMetricKind, BoundQuery, MetricPath};
use menura_representations::logical_plan::{AggregateComponent, DerivedFormula};
use menura_representations::names::ColumnName;
use menura_representations::paths::{Branch, Branches, ComponentKey};

use crate::columns::{component_column, over_scan, source_prefix};
use crate::dimensions::at;
use crate::error::{InvariantViolation, PlanError};
use crate::expansion::{ExpandedMetric, InputColumn, expand};

/// The model location of the first metric of `metrics`, where the final `Project` comes
/// from; `None` without metrics.
pub(crate) fn first_metric_location(metrics: &[PlannedMetric<'_>]) -> Option<SourceLocation> {
    metrics.first().map(|metric| metric.model.location.clone())
}

/// A metric of the query, read from the model and expanded into its base components.
pub(crate) struct PlannedMetric<'input> {
    bound: &'input BoundMetric,
    model: &'input Metric,
    expanded: ExpandedMetric<'input>,
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
    /// The derived metric whose formula it is, when it has a division rule (DER-3.1).
    pub(crate) derived: Option<DerivedFormula>,
}

/// The metrics of `query`, in output order, each read from `model`.
///
/// # Errors
///
/// An internal error for an inline metric, a metric or input the model does not have or did
/// not classify, a cycle of derived metrics, or a metric or input with the context extension.
pub(crate) fn planned_metrics<'input>(
    query: &'input BoundQuery,
    model: &'input InternalModel,
) -> Result<Vec<PlannedMetric<'input>>, PlanError> {
    query
        .columns()
        .metrics()
        .enumerate()
        .map(|(position, bound)| {
            planned_metric(bound, position, model).map_err(|violation| at(violation, &bound.origin))
        })
        .collect()
}

fn planned_metric<'input>(
    bound: &'input BoundMetric,
    position: usize,
    model: &'input InternalModel,
) -> Result<PlannedMetric<'input>, InvariantViolation> {
    let BoundMetricKind::Reference(path) = &bound.kind else {
        return Err(InvariantViolation::InlineMetric);
    };
    let found = model_metric(path, model).ok_or(InvariantViolation::UnknownMetric)?;
    Ok(PlannedMetric {
        bound,
        model: found,
        expanded: expand(path, position, model, &check_extensions)?,
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
    if metric.context.is_some() {
        return Err(InvariantViolation::ContextMetric);
    }
    Ok(())
}

/// The semi-additive declaration of the base metric whose components `branch` holds, with
/// that metric (E3-4.1).
///
/// # Errors
///
/// An internal error for a branch without components, or whose first component's metric is
/// not semi-additive.
pub(crate) fn semi_additive_metric<'input>(
    branch: &Branch,
    metrics: &[PlannedMetric<'input>],
) -> Result<(&'input Metric, &'input SemiAdditive), PlanError> {
    let unknown = || {
        PlanError::invariant(
            InvariantViolation::UnknownComponent,
            &crate::slice::query_root(),
        )
    };
    let key = branch.components.first().ok_or_else(unknown)?;
    let metric = metrics
        .get(key.metric)
        .and_then(|metric| metric.expanded.components.get(key.component))
        .map(|base| base.metric)
        .ok_or_else(unknown)?;
    match &metric.declared_additivity {
        Some(DeclaredAdditivity::SemiAdditive(semi_additive)) => Ok((metric, semi_additive)),
        Some(DeclaredAdditivity::NonAdditive) | None => Err(unknown()),
    }
}

/// The Aggregate component of every key of `branch`, in branch order, over the `Scan`
/// columns of `home`.
///
/// # Errors
///
/// An internal error for a key that names no component, or a component of another dataset.
pub(crate) fn branch_components(
    branch: &Branch,
    metrics: &[PlannedMetric<'_>],
    home: &Dataset,
) -> Result<Vec<AggregateComponent>, PlanError> {
    let prefix = source_prefix(home);
    branch
        .components
        .iter()
        .map(|key| component(*key, metrics, home, &prefix))
        .collect()
}

fn component(
    key: ComponentKey,
    metrics: &[PlannedMetric<'_>],
    home: &Dataset,
    prefix: &str,
) -> Result<AggregateComponent, PlanError> {
    let unknown = || {
        PlanError::invariant(
            InvariantViolation::UnknownComponent,
            &crate::slice::query_root(),
        )
    };
    let metric = metrics.get(key.metric).ok_or_else(unknown)?;
    let fail = |violation| at(violation, &metric.bound.origin);
    let base = metric
        .expanded
        .components
        .get(key.component)
        .ok_or_else(|| fail(InvariantViolation::UnknownComponent))?;
    let found = base.component;
    if found.home_dataset != home.name {
        return Err(fail(InvariantViolation::ForeignField));
    }
    let aggregate = Expression::Aggregate {
        call: found.call.clone(),
        filter: found.filter.clone().map(Box::new),
    };
    let Expression::Aggregate { call, filter } =
        over_scan(&aggregate, base.in_dataset_metric.then_some(prefix)).map_err(fail)?
    else {
        return Err(fail(InvariantViolation::ComponentMismatch));
    };
    Ok(AggregateComponent {
        name: component_column(key.metric, key.component),
        call,
        filter: filter.map(|filter| *filter),
    })
}

/// Every metric, in output order, evaluated from its components, which `branches` compute
/// (E4-5.0c).
///
/// # Errors
///
/// An internal error for a metric whose aggregates do not match its components, a component
/// that no branch computes, or a metric without a type.
pub(crate) fn metric_outputs(
    branches: &Branches,
    metrics: &[PlannedMetric<'_>],
) -> Result<Vec<MetricOutput>, PlanError> {
    let computed: Vec<ComponentKey> = branches
        .as_slice()
        .iter()
        .flat_map(|branch| branch.components.iter().copied())
        .collect();
    metrics
        .iter()
        .enumerate()
        .map(|(position, metric)| {
            metric_output(&computed, position, metric)
                .map_err(|violation| at(violation, &metric.bound.origin))
        })
        .collect()
}

fn metric_output(
    computed: &[ComponentKey],
    position: usize,
    metric: &PlannedMetric<'_>,
) -> Result<MetricOutput, InvariantViolation> {
    let every_computed = (0..metric.expanded.components.len()).all(|component| {
        computed.contains(&ComponentKey {
            metric: position,
            component,
        })
    });
    if !every_computed {
        return Err(InvariantViolation::UnknownComponent);
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
        expression: metric.expanded.expression.clone(),
        logical_type: logical_type.clone(),
        nullable: typed.nullable,
        derived: metric.expanded.derived.clone(),
    })
}

/// The input columns of every derived metric of `metrics`, in output order (DER-3.1).
pub(crate) fn input_columns<'plan>(metrics: &'plan [PlannedMetric<'_>]) -> Vec<&'plan InputColumn> {
    metrics
        .iter()
        .flat_map(|metric| metric.expanded.inputs.iter())
        .collect()
}

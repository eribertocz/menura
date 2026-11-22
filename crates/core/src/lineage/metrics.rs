//! The metrics an output metric derives from, transitively (PLN-4.4).

use std::collections::BTreeSet;

use menura_model::expression::Expression;
use menura_model::internal_model::{InternalModel, Metric, NormalizedName};
use menura_representations::bound_query::{BoundMetric, BoundMetricKind, MetricPath};

use super::references::{aggregated_references, normalized_name};
use crate::compile::InternalError;

/// The metrics `metric` derives from: itself when it is a model metric, and every metric it
/// reads, directly or through other metrics: the `AGG(…)` references of its expression or of
/// an inline metric, the inputs of a derived metric and the base of a context metric. By
/// normalized name.
///
/// # Errors
///
/// An [`InternalError`] for a metric the bound query or a metric names and the model does
/// not have.
pub(super) fn metrics_read(
    metric: &BoundMetric,
    model: &InternalModel,
) -> Result<BTreeSet<NormalizedName>, InternalError> {
    let mut visited = BTreeSet::new();
    match &metric.kind {
        BoundMetricKind::Reference(path) => add_metric(path, model, &mut visited)?,
        BoundMetricKind::Inline(expression) => {
            for path in aggregated_metrics(expression) {
                add_metric(&path, model, &mut visited)?;
            }
        }
    }
    Ok(visited
        .into_iter()
        .map(|path| name(&path).clone())
        .collect())
}

/// Adds the metric `path` and every metric it reads to `visited`, once each.
fn add_metric(
    path: &MetricPath,
    model: &InternalModel,
    visited: &mut BTreeSet<MetricPath>,
) -> Result<(), InternalError> {
    if !visited.insert(path.clone()) {
        return Ok(());
    }
    let metric = find(path, model).ok_or_else(|| unknown(name(path)))?;
    for input in inputs(metric, path, model) {
        add_metric(&input, model, visited)?;
    }
    Ok(())
}

/// The metrics `metric`, at `path`, reads directly: the `AGG(…)` references of its expression,
/// the inputs of a derived metric, which are metrics of the semantic model, and the base of a
/// context metric, which is a metric of its own dataset when that dataset has one of that
/// name, otherwise a metric of the semantic model.
fn inputs(metric: &Metric, path: &MetricPath, model: &InternalModel) -> Vec<MetricPath> {
    let mut inputs: Vec<MetricPath> = metric
        .expression
        .as_ref()
        .map(aggregated_metrics)
        .unwrap_or_default();
    inputs.extend(
        metric
            .derived
            .iter()
            .flat_map(|derived| derived.inputs.iter().cloned())
            .map(MetricPath::SemanticModel),
    );
    if let Some(context) = &metric.context {
        let own = match path {
            MetricPath::Dataset { dataset, .. } => Some(MetricPath::Dataset {
                dataset: dataset.clone(),
                metric: context.base.clone(),
            }),
            MetricPath::SemanticModel(_) => None,
        };
        inputs.push(match own {
            Some(own) if find(&own, model).is_some() => own,
            _ => MetricPath::SemanticModel(context.base.clone()),
        });
    }
    inputs
}

/// The metrics `expression` reads with `AGG(…)`: `AGG(dataset.metric)` a dataset metric,
/// `AGG(metric)` a metric of the semantic model (DER-5.1.2).
fn aggregated_metrics(expression: &Expression) -> Vec<MetricPath> {
    aggregated_references(expression)
        .into_iter()
        .map(|reference| {
            let metric = normalized_name(&reference.name);
            match &reference.qualifier {
                Some(dataset) => MetricPath::Dataset {
                    dataset: normalized_name(dataset),
                    metric,
                },
                None => MetricPath::SemanticModel(metric),
            }
        })
        .collect()
}

/// The metric at `path`.
fn find<'model>(path: &MetricPath, model: &'model InternalModel) -> Option<&'model Metric> {
    match path {
        MetricPath::SemanticModel(name) => model.metric(name),
        MetricPath::Dataset { dataset, metric } => model
            .dataset(dataset)?
            .metrics
            .iter()
            .find(|found| &found.name == metric),
    }
}

/// The name of the metric at `path`.
fn name(path: &MetricPath) -> &NormalizedName {
    match path {
        MetricPath::SemanticModel(name) | MetricPath::Dataset { metric: name, .. } => name,
    }
}

/// The defect of a plan whose query or model names the metric `name` that the model does not
/// have.
fn unknown(name: &NormalizedName) -> InternalError {
    InternalError::new(
        "core",
        format!(
            "the query reads the metric `{}`, which the model does not have",
            name.as_str()
        ),
    )
}

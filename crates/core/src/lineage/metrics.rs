//! The metrics an output metric derives from, transitively (PLN-4.4).

use std::collections::BTreeSet;

use menura_model::internal_model::{InternalModel, Metric, NormalizedName};
use menura_representations::bound_query::{BoundMetric, BoundMetricKind, MetricPath};

use super::references::{aggregated_references, normalized_name};
use crate::compile::InternalError;

/// The metrics `metric` derives from: itself when it is a model metric, and every metric it
/// reads, directly or through other metrics: the inputs of a derived metric, the `AGG(…)`
/// references of an inline metric and the base of a context metric. By normalized name.
///
/// # Errors
///
/// An [`InternalError`] for a metric the bound query or a metric names and the model does
/// not have.
pub(super) fn metrics_read(
    metric: &BoundMetric,
    model: &InternalModel,
) -> Result<BTreeSet<NormalizedName>, InternalError> {
    let mut found = BTreeSet::new();
    match &metric.kind {
        BoundMetricKind::Reference(MetricPath::SemanticModel(name)) => {
            add_metric(name, model, &mut found)?;
        }
        BoundMetricKind::Reference(MetricPath::Dataset { dataset, metric }) => {
            let declared = model
                .dataset(dataset)
                .and_then(|dataset| dataset.metrics.iter().find(|found| &found.name == metric))
                .ok_or_else(|| unknown(metric))?;
            found.insert(declared.name.clone());
            add_inputs(declared, model, &mut found)?;
        }
        BoundMetricKind::Inline(expression) => {
            for reference in aggregated_references(expression) {
                add_metric(&normalized_name(&reference.name), model, &mut found)?;
            }
        }
    }
    Ok(found)
}

/// Adds the model metric `name` and every metric it reads to `found`, once each.
fn add_metric(
    name: &NormalizedName,
    model: &InternalModel,
    found: &mut BTreeSet<NormalizedName>,
) -> Result<(), InternalError> {
    if found.contains(name) {
        return Ok(());
    }
    let metric = model.metric(name).ok_or_else(|| unknown(name))?;
    found.insert(metric.name.clone());
    add_inputs(metric, model, found)
}

/// Adds every metric `metric` reads, and those they read, to `found`.
fn add_inputs(
    metric: &Metric,
    model: &InternalModel,
    found: &mut BTreeSet<NormalizedName>,
) -> Result<(), InternalError> {
    let derived = metric.derived.iter().flat_map(|derived| &derived.inputs);
    let classified = metric
        .classification
        .iter()
        .flat_map(|classification| &classification.inputs);
    let context = metric.context.iter().map(|context| &context.base);
    let inputs: BTreeSet<&NormalizedName> = derived.chain(classified).chain(context).collect();
    for input in inputs {
        add_metric(input, model, found)?;
    }
    Ok(())
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

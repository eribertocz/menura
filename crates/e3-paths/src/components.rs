//! The components of the metrics of a bound query, read from the internal model.

use menura_model::internal_model::{Component, InternalModel, Metric};
use menura_representations::bound_query::{BoundMetric, BoundMetricKind, MetricPath};

use crate::violation::InvariantViolation;

/// The components of `metric`, in written order (E0G-1.1): at least one.
///
/// # Errors
///
/// [`InvariantViolation::InlineMetric`] for an inline metric,
/// [`InvariantViolation::UnclassifiedMetric`] for a metric the model does not have or did not
/// classify, and [`InvariantViolation::MetricWithoutComponents`] for a metric with none.
pub(crate) fn components<'model>(
    metric: &BoundMetric,
    model: &'model InternalModel,
) -> Result<&'model [Component], InvariantViolation> {
    let origin = || metric.origin.clone();
    let path = match &metric.kind {
        BoundMetricKind::Reference(path) => path,
        BoundMetricKind::Inline(_) => {
            return Err(InvariantViolation::InlineMetric { origin: origin() });
        }
    };
    let classification = model_metric(path, model)
        .and_then(|model_metric| model_metric.classification.as_ref())
        .ok_or_else(|| InvariantViolation::UnclassifiedMetric { origin: origin() })?;
    if classification.components.is_empty() {
        return Err(InvariantViolation::MetricWithoutComponents { origin: origin() });
    }
    Ok(&classification.components)
}

/// The metric of the model that `path` names, when there is one.
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

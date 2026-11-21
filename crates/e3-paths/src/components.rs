//! The components of the metrics of a bound query, read from the internal model.

use menura_model::internal_model::{Component, InternalModel};
use menura_representations::bound_query::{BoundMetric, BoundMetricKind};

use crate::derived::{ExpansionFailure, base_components};
use crate::violation::InvariantViolation;

/// The components of `metric`, at least one: its own in written order (E0G-1.1), or for a
/// derived metric those of its inputs, expanded down to base metrics (DER-2.1, DER-2.4).
///
/// # Errors
///
/// [`InvariantViolation::InlineMetric`] for an inline metric,
/// [`InvariantViolation::UnclassifiedMetric`] for a metric, or an input, the model does not
/// have or did not classify, [`InvariantViolation::DerivedCycle`] for inputs that lead back to
/// a derived metric, and [`InvariantViolation::MetricWithoutComponents`] for a metric with
/// none.
pub(crate) fn components<'model>(
    metric: &BoundMetric,
    model: &'model InternalModel,
) -> Result<Vec<&'model Component>, InvariantViolation> {
    let origin = || metric.origin.clone();
    let path = match &metric.kind {
        BoundMetricKind::Reference(path) => path,
        BoundMetricKind::Inline(_) => {
            return Err(InvariantViolation::InlineMetric { origin: origin() });
        }
    };
    let found = base_components(path, model).map_err(|failure| match failure {
        ExpansionFailure::UnknownMetric => {
            InvariantViolation::UnclassifiedMetric { origin: origin() }
        }
        ExpansionFailure::Cycle => InvariantViolation::DerivedCycle { origin: origin() },
    })?;
    if found.is_empty() {
        return Err(InvariantViolation::MetricWithoutComponents { origin: origin() });
    }
    Ok(found)
}

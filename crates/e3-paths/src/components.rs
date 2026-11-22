//! The components of the metrics of a bound query, read from the internal model.

use menura_model::internal_model::InternalModel;
use menura_representations::bound_query::{BoundMetric, BoundMetricKind};

use crate::derived::{BaseComponent, ExpansionFailure, base_components, inline_components};
use crate::violation::InvariantViolation;

/// The components of `metric`, each with its base metric, at least one: its own in written order (E0G-1.1), or for a
/// derived metric those of its inputs, expanded down to base metrics (DER-2.1, DER-2.4); an
/// inline metric of the SQL form expands as a derived metric with its formula (E1-4.2).
///
/// # Errors
///
/// [`InvariantViolation::UnclassifiedMetric`] for a metric, or an input, the model does not
/// have or did not classify, [`InvariantViolation::DerivedCycle`] for inputs that lead back to
/// a derived metric, and [`InvariantViolation::MetricWithoutComponents`] for a metric with
/// none.
pub(crate) fn components<'model>(
    metric: &BoundMetric,
    model: &'model InternalModel,
) -> Result<Vec<BaseComponent<'model>>, InvariantViolation> {
    let origin = || metric.origin.clone();
    let expanded = match &metric.kind {
        BoundMetricKind::Reference(path) => base_components(path, model),
        BoundMetricKind::Inline(formula) => inline_components(formula, model),
    };
    let found = expanded.map_err(|failure| match failure {
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

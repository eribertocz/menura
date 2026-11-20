//! The model elements a bound name points to.

use menura_model::internal_model::{DiagnosticCode, InternalModel, Metric};
use menura_representations::bound_query::MetricPath;

use crate::authorized_query::ResolvedName;
use crate::named::NamedTarget;

/// The metric `path` names.
pub(crate) fn metric<'model>(
    model: &'model InternalModel,
    path: &MetricPath,
) -> Option<&'model Metric> {
    match path {
        MetricPath::SemanticModel(name) => model.metric(name),
        MetricPath::Dataset { dataset, metric } => model
            .dataset(dataset)?
            .metrics
            .iter()
            .find(|candidate| &candidate.name == metric),
    }
}

/// The query error codes of `target` and of the dataset that holds it (E0-4.1, E0-4.2).
pub(crate) fn query_errors<'model>(
    model: &'model InternalModel,
    target: &NamedTarget,
) -> Vec<&'model DiagnosticCode> {
    let dataset_errors = |name| {
        model
            .dataset(name)
            .map(|dataset| dataset.query_errors.iter().collect::<Vec<_>>())
            .unwrap_or_default()
    };
    match target {
        NamedTarget::Member(ResolvedName::Field(path)) => {
            let mut codes = dataset_errors(&path.dataset);
            if let Some(field) = model
                .dataset(&path.dataset)
                .and_then(|dataset| dataset.field(&path.field))
            {
                codes.extend(&field.query_errors);
            }
            codes
        }
        NamedTarget::Member(ResolvedName::Metric(path)) => {
            let mut codes = match path {
                MetricPath::Dataset { dataset, .. } => dataset_errors(dataset),
                MetricPath::SemanticModel(_) => Vec::new(),
            };
            if let Some(found) = metric(model, path) {
                codes.extend(&found.query_errors);
            }
            codes
        }
        NamedTarget::Dataset(name) => dataset_errors(name),
        NamedTarget::Relationship(name) => model
            .relationship(name)
            .map(|relationship| relationship.query_errors.iter().collect())
            .unwrap_or_default(),
    }
}

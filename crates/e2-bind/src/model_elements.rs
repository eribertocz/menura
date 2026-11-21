//! The model elements a bound name points to.

use menura_model::internal_model::{
    DiagnosticCode, ElementKind, InternalModel, Metric, NormalizedName, RowAccessCut,
};
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

/// The cause of `E0H-01` that E0 recorded for `target`, or else for the dataset that holds it
/// (E0-4.2a).
pub(crate) fn row_access_cut<'model>(
    model: &'model InternalModel,
    target: &NamedTarget,
) -> Option<&'model RowAccessCut> {
    let recorded = |kind: ElementKind, name: Vec<&NormalizedName>| {
        model
            .parts()
            .unusable
            .iter()
            .find(|element| element.kind == kind && element.name.iter().eq(name.iter().copied()))
            .and_then(|element| element.row_access_cut.as_ref())
    };
    match target {
        NamedTarget::Member(ResolvedName::Field(path)) => {
            recorded(ElementKind::Field, vec![&path.dataset, &path.field])
                .or_else(|| recorded(ElementKind::Dataset, vec![&path.dataset]))
        }
        NamedTarget::Member(ResolvedName::Metric(MetricPath::Dataset { dataset, metric })) => {
            recorded(ElementKind::Metric, vec![dataset, metric])
                .or_else(|| recorded(ElementKind::Dataset, vec![dataset]))
        }
        NamedTarget::Member(ResolvedName::Metric(MetricPath::SemanticModel(name))) => {
            recorded(ElementKind::Metric, vec![name])
        }
        NamedTarget::Dataset(name) => recorded(ElementKind::Dataset, vec![name]),
        NamedTarget::Relationship(name) => recorded(ElementKind::Relationship, vec![name]),
    }
}

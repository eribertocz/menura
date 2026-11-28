//! Which datasets, fields and metrics a description lists (DSC-3.1, DSC-3.2, DSC-3.7).

use menura_contracts::types::audit_record::Access;
use menura_contracts::types::common::LogicalType;
use menura_model::internal_model::{
    Dataset, Field, FieldPath, InternalModel, Metric, NormalizedName, ValueType,
};
use menura_representations::bound_query::MetricPath;
use menura_security::Authorization;

/// The elements a description lists, each ordered by normalized name (DSC-3.4).
pub(super) struct Listing<'model> {
    /// The listed datasets, with their listed fields.
    pub(super) datasets: Vec<ListedDataset<'model>>,
    /// The listed metrics: of the semantic model, then dataset-scoped, unordered.
    pub(super) metrics: Vec<ListedMetric<'model>>,
}

/// A listed dataset.
pub(super) struct ListedDataset<'model> {
    /// The dataset.
    pub(super) dataset: &'model Dataset,
    /// Its listed fields, with their logical types, ordered by name.
    pub(super) fields: Vec<(&'model Field, &'model LogicalType)>,
}

/// A listed metric.
pub(super) struct ListedMetric<'model> {
    /// Its reference: `metric`, or `dataset.metric` for a dataset-scoped metric.
    pub(super) reference: String,
    /// The metric.
    pub(super) metric: &'model Metric,
    /// Its result type.
    pub(super) logical_type: &'model LogicalType,
    /// The dataset whose relation exposes it to a SQL-form query, if any (QRY-5.2).
    pub(super) exposed_on: Option<NormalizedName>,
}

impl<'model> Listing<'model> {
    /// What the user `authorization` decides on may see of `model`: every authorized, usable
    /// dataset that is not a summary, its authorized, usable fields whose type is supported,
    /// and every authorized, usable metric.
    pub(super) fn new(model: &'model InternalModel, authorization: &Authorization<'_>) -> Self {
        let datasets: Vec<ListedDataset<'model>> = model
            .parts()
            .datasets
            .iter()
            .filter(|dataset| is_listed_dataset(dataset, authorization))
            .map(|dataset| ListedDataset {
                dataset,
                fields: listed_fields(dataset, authorization),
            })
            .collect();
        let semantic = model.parts().metrics.iter().map(|metric| {
            let path = MetricPath::SemanticModel(metric.name.clone());
            let exposed_on = sole_home_dataset(metric);
            (metric.name.as_str().to_owned(), path, metric, exposed_on)
        });
        let scoped = datasets.iter().flat_map(|listed| {
            let dataset = &listed.dataset.name;
            listed.dataset.metrics.iter().map(move |metric| {
                let path = MetricPath::Dataset {
                    dataset: dataset.clone(),
                    metric: metric.name.clone(),
                };
                let reference = format!("{}.{}", dataset.as_str(), metric.name.as_str());
                (reference, path, metric, Some(dataset.clone()))
            })
        });
        let metrics = semantic
            .chain(scoped)
            .filter(|(_, path, _, _)| authorization.metric(path).access == Access::Granted)
            .filter_map(|(reference, _, metric, exposed_on)| {
                listed_metric(reference, metric, exposed_on)
            })
            .collect();
        Self { datasets, metrics }
    }

    /// Whether the dataset `name` is listed.
    pub(super) fn lists_dataset(&self, name: &NormalizedName) -> bool {
        self.datasets
            .binary_search_by(|listed| listed.dataset.name.cmp(name))
            .is_ok()
    }
}

/// Whether `dataset` is listed: authorized (DSC-3.1), not a summary (DSC-3.2), usable
/// (DSC-3.7).
fn is_listed_dataset(dataset: &Dataset, authorization: &Authorization<'_>) -> bool {
    !dataset.summary
        && dataset.query_errors.is_empty()
        && authorization.dataset(&dataset.name).access == Access::Granted
}

/// The fields of `dataset` that are listed: authorized (DSC-3.1), usable, and of a supported
/// logical type (DSC-3.7).
fn listed_fields<'model>(
    dataset: &'model Dataset,
    authorization: &Authorization<'_>,
) -> Vec<(&'model Field, &'model LogicalType)> {
    dataset
        .fields
        .iter()
        .filter(|field| field.query_errors.is_empty())
        .filter(|field| {
            let path = FieldPath {
                dataset: dataset.name.clone(),
                field: field.name.clone(),
            };
            authorization.field(&path).access == Access::Granted
        })
        .filter_map(|field| {
            logical_type(field.typed.as_ref().map(|typed| &typed.value_type))
                .map(|logical_type| (field, logical_type))
        })
        .collect()
}

/// The dataset a metric of the semantic model is exposed on in the SQL form: its home dataset,
/// when every component has that one (QRY-5.2, DEV-8).
fn sole_home_dataset(metric: &Metric) -> Option<NormalizedName> {
    match metric.classification.as_ref()?.home_datasets.as_slice() {
        [home] => Some(home.clone()),
        _ => None,
    }
}

/// The listed metric `metric` under `reference`, exposed on `exposed_on`, when it is usable
/// (DSC-3.7) and typed.
fn listed_metric(
    reference: String,
    metric: &Metric,
    exposed_on: Option<NormalizedName>,
) -> Option<ListedMetric<'_>> {
    if !metric.query_errors.is_empty() || metric.classification.is_none() {
        return None;
    }
    let logical_type = logical_type(metric.typed.as_ref().map(|typed| &typed.value_type))?;
    Some(ListedMetric {
        reference,
        metric,
        logical_type,
        exposed_on,
    })
}

/// The logical type of `value_type`, unless it is missing, the type of `NULL`, or
/// `unsupported` (DSC-3.7).
fn logical_type(value_type: Option<&ValueType>) -> Option<&LogicalType> {
    match value_type? {
        ValueType::Logical(LogicalType::Unsupported) | ValueType::Null => None,
        ValueType::Logical(logical_type) => Some(logical_type),
    }
}

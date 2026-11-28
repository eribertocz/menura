//! The `datasets` member of a description (DSC-2, DSC-3.8, DSC-3.11).

use menura_contracts::Sha256Hash;
use menura_contracts::compare_names;
use menura_contracts::types::common::{Grain, LogicalType};
use menura_contracts::types::descriptions::{DescribedDataset, DescribedField};
use menura_model::internal_model::{Dataset, Field};

use super::ai_context::ai_context;
use super::listing::{ListedDataset, Listing};

/// Every grain a date or timestamp field accepts, in the order DSC-3.8 lists them.
const TIME_GRAINS: [Grain; 5] = [
    Grain::Day,
    Grain::Week,
    Grain::Month,
    Grain::Quarter,
    Grain::Year,
];

/// The listed datasets of `listing`, ordered by name, in the model of `model_hash`.
pub(super) fn described_datasets(
    listing: &Listing<'_>,
    model_hash: &Sha256Hash,
) -> Vec<DescribedDataset> {
    listing
        .datasets
        .iter()
        .map(|listed| described_dataset(listed, listing, model_hash))
        .collect()
}

fn described_dataset(
    listed: &ListedDataset<'_>,
    listing: &Listing<'_>,
    model_hash: &Sha256Hash,
) -> DescribedDataset {
    let dataset = listed.dataset;
    DescribedDataset {
        name: dataset.name.as_str().to_owned(),
        label: dataset.descriptions.label.clone(),
        description: dataset.descriptions.description.clone(),
        fields: listed
            .fields
            .iter()
            .map(|(field, logical_type)| described_field(dataset, field, logical_type, model_hash))
            .collect(),
        ai_context: ai_context(&dataset.descriptions, &dataset.location, model_hash),
        sql_metrics: Some(sql_metrics(dataset, listing)),
    }
}

/// The references of the listed metrics exposed on the relation of `dataset`, in the order of
/// the description's `metrics` (DSC-3.1, DSC-3.4, QRY-5.2).
fn sql_metrics(dataset: &Dataset, listing: &Listing<'_>) -> Vec<String> {
    let mut references: Vec<String> = listing
        .metrics
        .iter()
        .filter(|listed| listed.exposed_on.as_ref() == Some(&dataset.name))
        .map(|listed| listed.reference.clone())
        .collect();
    references.sort_by(|left, right| compare_names(left, right));
    references
}

fn described_field(
    dataset: &Dataset,
    field: &Field,
    logical_type: &LogicalType,
    model_hash: &Sha256Hash,
) -> DescribedField {
    DescribedField {
        reference: format!("{}.{}", dataset.name.as_str(), field.name.as_str()),
        label: field.descriptions.label.clone(),
        description: field.descriptions.description.clone(),
        logical_type: logical_type.clone(),
        is_time: field.is_time.unwrap_or(false),
        time_grains: time_grains(logical_type),
        ai_context: ai_context(&field.descriptions, &field.location, model_hash),
    }
}

/// Every grain for a date or timestamp, none for any other type (DSC-3.8).
fn time_grains(logical_type: &LogicalType) -> Vec<Grain> {
    match logical_type {
        LogicalType::Date | LogicalType::Timestamp { .. } => TIME_GRAINS.to_vec(),
        _ => Vec::new(),
    }
}

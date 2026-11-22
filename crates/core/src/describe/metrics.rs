//! The `metrics` member of a description (DSC-2, E3-3.4).

use menura_contracts::Sha256Hash;
use menura_contracts::compare_names;
use menura_contracts::types::descriptions::{DescribedMetric, MetricKind};
use menura_model::internal_model::Metric;

use super::ai_context::ai_context;
use super::graph::DescribedGraph;
use super::listing::{ListedMetric, Listing};

/// The listed metrics of `listing`, ordered by reference (DSC-3.4), with the datasets each
/// reaches over `graph`, in the model of `model_hash`.
pub(super) fn described_metrics(
    listing: &Listing<'_>,
    graph: &DescribedGraph<'_>,
    model_hash: &Sha256Hash,
) -> Vec<DescribedMetric> {
    let mut metrics: Vec<DescribedMetric> = listing
        .metrics
        .iter()
        .map(|listed| described_metric(listed, graph, model_hash))
        .collect();
    metrics.sort_by(|left, right| compare_names(&left.reference, &right.reference));
    metrics
}

fn described_metric(
    listed: &ListedMetric<'_>,
    graph: &DescribedGraph<'_>,
    model_hash: &Sha256Hash,
) -> DescribedMetric {
    let metric = listed.metric;
    let homes = metric
        .classification
        .as_ref()
        .map(|classification| classification.home_datasets.as_slice())
        .unwrap_or_default();
    DescribedMetric {
        reference: listed.reference.clone(),
        label: metric.descriptions.label.clone(),
        description: metric.descriptions.description.clone(),
        logical_type: listed.logical_type.clone(),
        kind: kind(metric),
        reachable_datasets: graph.reached_by_all(homes),
        ai_context: ai_context(&metric.descriptions, &metric.location, model_hash),
    }
}

/// `derived` for a metric of the derived-metric extension, `context` for one of the context
/// extension, `base` otherwise.
fn kind(metric: &Metric) -> MetricKind {
    if metric.derived.is_some() {
        MetricKind::Derived
    } else if metric.context.is_some() {
        MetricKind::Context
    } else {
        MetricKind::Base
    }
}

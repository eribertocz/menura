//! SUM-3.2: the components that non-additive and semi-additive metrics of the query read.

use menura_model::expression::{Expression, Reference};
use menura_model::internal_model::{Additivity, InternalModel, Metric, NormalizedName};
use menura_representations::bound_query::{BoundMetricKind, BoundQuery, MetricPath};
use menura_representations::columns::OutputColumn;

use super::branch::BranchShape;
use super::equivalence::{ScanAggregate, over_scan};
use super::expression_map::{any_node, field_name};

/// Whether a component of the branch is, after normalization, a component that a
/// non-additive or semi-additive metric of the query reads, directly or through the inputs of
/// a derived metric, even if an additive metric reads it too (SUM-3.2, E4-7.9).
pub(super) fn reads_restricted_metric(
    shape: &BranchShape<'_>,
    model: &InternalModel,
    query: &BoundQuery,
) -> bool {
    let mut restricted = Vec::new();
    for metric in query_metrics(model, query) {
        visit(metric, false, model, &mut Vec::new(), &mut restricted);
    }
    let restricted: Vec<ScanAggregate> = restricted
        .into_iter()
        .filter(|(home, _)| home == shape.home)
        .map(|(_, aggregate)| aggregate)
        .collect();
    shape.components.iter().any(|component| {
        restricted
            .iter()
            .any(|(call, filter)| call == &component.call && filter == &component.filter)
    })
}

/// The model metrics the query names, and those its inline metrics read with `AGG(…)`.
fn query_metrics<'model>(model: &'model InternalModel, query: &BoundQuery) -> Vec<&'model Metric> {
    let mut metrics = Vec::new();
    for column in query.columns().iter() {
        let OutputColumn::Metric(bound) = column else {
            continue;
        };
        match &bound.kind {
            BoundMetricKind::Reference(path) => metrics.extend(model_metric(model, path)),
            BoundMetricKind::Inline(expression) => {
                metrics.extend(inline_inputs(model, expression));
            }
        }
    }
    metrics
}

fn model_metric<'model>(model: &'model InternalModel, path: &MetricPath) -> Option<&'model Metric> {
    match path {
        MetricPath::SemanticModel(name) => model.metric(name),
        MetricPath::Dataset { dataset, metric } => model
            .dataset(dataset)?
            .metrics
            .iter()
            .find(|candidate| &candidate.name == metric),
    }
}

/// The metrics an inline metric reads with `AGG(…)`: a qualified reference names a dataset
/// metric, an unqualified one a metric of the semantic model.
fn inline_inputs<'model>(
    model: &'model InternalModel,
    expression: &Expression,
) -> Vec<&'model Metric> {
    let mut read: Vec<Reference> = Vec::new();
    any_node(expression, &mut |node| {
        if let Expression::MetricAggregate(reference) = node {
            read.push(reference.clone());
        }
        false
    });
    read.iter()
        .filter_map(|reference| {
            let metric = NormalizedName::new(&field_name(&reference.name));
            let path = match &reference.qualifier {
                Some(dataset) => MetricPath::Dataset {
                    dataset: NormalizedName::new(&field_name(dataset)),
                    metric,
                },
                None => MetricPath::SemanticModel(metric),
            };
            model_metric(model, &path)
        })
        .collect()
}

/// Adds to `found` the components of `metric`, with their home dataset, when it or a derived
/// metric that reads it is restricted, then visits its inputs.
fn visit<'model>(
    metric: &'model Metric,
    read_by_restricted: bool,
    model: &'model InternalModel,
    visited: &mut Vec<&'model NormalizedName>,
    found: &mut Vec<(NormalizedName, ScanAggregate)>,
) {
    if visited.contains(&&metric.name) {
        return;
    }
    visited.push(&metric.name);
    let restricted = read_by_restricted || is_restricted(metric);
    let Some(classification) = &metric.classification else {
        return;
    };
    if restricted {
        found.extend(classification.components.iter().filter_map(|component| {
            let aggregate = over_scan(&Expression::Aggregate {
                call: component.call.clone(),
                filter: component.filter.clone().map(Box::new),
            })?;
            Some((component.home_dataset.clone(), aggregate))
        }));
    }
    for input in &classification.inputs {
        if let Some(input) = model.metric(input) {
            visit(input, restricted, model, visited, found);
        }
    }
}

/// Whether `metric` is declared or deduced non-additive or semi-additive (E4-7.9, ADD-3).
fn is_restricted(metric: &Metric) -> bool {
    metric.declared_additivity.is_some()
        || metric
            .classification
            .as_ref()
            .is_some_and(|classification| classification.additivity != Additivity::Additive)
}

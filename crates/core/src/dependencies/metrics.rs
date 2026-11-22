//! The inputs of a metric (DEP-3.7): every metric it reads, transitively, and every field those
//! metrics read, with the inputs of each such field.

use std::collections::BTreeSet;

use menura_model::expression::{Expression, Identifier};
use menura_model::internal_model::{
    ContextModifier, DeclaredAdditivity, DiagnosticCode, FieldPath, Metric, NormalizedName,
    RemovedDimensions, SemiAdditive,
};

use super::inputs::Inputs;
use super::names::{MetricName, ModelInput};
use super::resolve::{Resolver, row_paths};
use crate::lineage::references::{aggregated_references, normalized_name, plain_references};

/// The qualifier of a catalog column in a dataset metric expression, compared as a regular
/// identifier (E0D-3).
const SOURCE_QUALIFIER: &str = "SOURCE";

/// The metric `name` of the model: of the model, or of a dataset.
pub(super) fn find<'model>(
    resolver: &Resolver<'model>,
    name: &MetricName,
) -> Option<&'model Metric> {
    let model = resolver.model();
    match &name.second {
        None => model.metric(&name.first),
        Some(second) => model
            .dataset(&name.first)?
            .metrics
            .iter()
            .find(|metric| &metric.name == second),
    }
}

/// The inputs of the metric `name`, the metric itself included (DEP-3.7).
pub(super) fn metric_inputs(resolver: &Resolver<'_>, name: &MetricName) -> Inputs {
    let mut inputs = Inputs::default();
    let mut seen: BTreeSet<MetricName> = BTreeSet::new();
    let mut pending = vec![name.clone()];
    while let Some(current) = pending.pop() {
        if !seen.insert(current.clone()) {
            continue;
        }
        let Some(metric) = find(resolver, &current) else {
            continue;
        };
        inputs.model.insert(ModelInput::Metric(current.clone()));
        let reads = reads(metric, current.second.as_ref().map(|_| &current.first));
        for path in reads
            .fields
            .iter()
            .chain(&population_fields(resolver, metric))
        {
            inputs.absorb(&resolver.field(path));
        }
        if let Some(dataset) = &current.second.as_ref().map(|_| &current.first) {
            inputs.absorb(&home_dataset(resolver, dataset, metric));
        }
        pending.extend(reads.metrics);
    }
    inputs
}

/// What one metric reads directly.
#[derive(Debug, Default)]
struct Reads {
    /// The fields it reads.
    fields: Vec<FieldPath>,
    /// The metrics it reads.
    metrics: Vec<MetricName>,
}

/// What `metric` reads directly: in its expression and `FILTER` conditions, its derived inputs
/// and context base, and the fields of its extensions. `dataset` is the dataset of a dataset
/// metric, whose bare references are fields of that dataset (E0D-3).
fn reads(metric: &Metric, dataset: Option<&NormalizedName>) -> Reads {
    let mut found = Reads::default();
    if let Some(expression) = &metric.expression {
        expression_reads(expression, dataset, &mut found);
    }
    if let Some(derived) = &metric.derived {
        found
            .metrics
            .extend(derived.inputs.iter().map(MetricName::of_model));
    }
    if let Some(context) = &metric.context {
        found.metrics.push(MetricName::of_model(&context.base));
        found.fields.extend(modifier_fields(&context.modifier));
    }
    if let Some(DeclaredAdditivity::SemiAdditive(keys)) = &metric.declared_additivity {
        found.fields.extend(semi_additive_fields(keys));
    }
    found
}

fn expression_reads(expression: &Expression, dataset: Option<&NormalizedName>, found: &mut Reads) {
    for reference in plain_references(expression) {
        let name = normalized_name(&reference.name);
        let qualifier = reference.qualifier.as_ref();
        match (dataset, qualifier) {
            (Some(dataset), None) => found.fields.push(FieldPath {
                dataset: dataset.clone(),
                field: name,
            }),
            (None, Some(qualifier)) => found.fields.push(FieldPath {
                dataset: normalized_name(qualifier),
                field: name,
            }),
            // A dataset metric's `source.column` reads a catalog column (`home_dataset`), and a
            // model metric's bare reference is outside the accepted subset (E0D-3.4).
            _ => {}
        }
    }
    for reference in aggregated_references(expression) {
        let name = normalized_name(&reference.name);
        found.metrics.push(match &reference.qualifier {
            Some(qualifier) => MetricName::of_dataset(&normalized_name(qualifier), &name),
            None => MetricName::of_model(&name),
        });
    }
}

/// The dataset of a dataset metric with its inputs, whose visible rows the metric reads
/// (DEP-3.5), and the catalog columns the metric reads as `source.column`.
fn home_dataset(resolver: &Resolver<'_>, dataset: &NormalizedName, metric: &Metric) -> Inputs {
    let mut inputs = resolver.dataset(dataset);
    inputs.model.insert(ModelInput::Dataset(dataset.clone()));
    let Some(found) = resolver.model().dataset(dataset) else {
        return inputs;
    };
    for reference in metric.expression.iter().flat_map(plain_references) {
        if reference.qualifier.as_ref().is_some_and(is_source) {
            inputs
                .physical
                .extend(resolver.column(found, reference.name.text()));
        }
    }
    inputs
}

/// Whether `qualifier` is `source`, compared as a regular identifier (E0D-3). E0 writes every
/// name of a tree in its normalized form.
fn is_source(qualifier: &Identifier) -> bool {
    normalized_name(qualifier).as_str() == SOURCE_QUALIFIER
}

/// The `time_dimension` or `remove` fields of a context modifier (CTX-2).
fn modifier_fields(modifier: &ContextModifier) -> Vec<FieldPath> {
    match modifier {
        ContextModifier::PeriodToDate { time_dimension, .. }
        | ContextModifier::PriorPeriod { time_dimension, .. } => vec![time_dimension.clone()],
        ContextModifier::ShareOfTotal(RemovedDimensions::Fields(fields)) => fields.clone(),
        ContextModifier::ShareOfTotal(RemovedDimensions::All) => Vec::new(),
    }
}

/// The `over` and `entity` fields of a semi-additive declaration (ADD-3.2). The fields of its
/// population are those of [`population_fields`].
fn semi_additive_fields(keys: &SemiAdditive) -> Vec<FieldPath> {
    std::iter::once(keys.over.clone())
        .chain(keys.entity.iter().cloned())
        .collect()
}

/// The fields of the population a semi-additive metric names: its `entity` and time fields,
/// and those `required_when` reads (POP-2).
fn population_fields(resolver: &Resolver<'_>, metric: &Metric) -> Vec<FieldPath> {
    let Some(DeclaredAdditivity::SemiAdditive(keys)) = &metric.declared_additivity else {
        return Vec::new();
    };
    let Some(wanted) = &keys.population else {
        return Vec::new();
    };
    let mut fields = Vec::new();
    for dataset in &resolver.model().parts().datasets {
        for population in dataset
            .populations
            .iter()
            .filter(|population| &population.name == wanted)
        {
            fields.extend(
                population
                    .named_fields()
                    .into_iter()
                    .map(|field| FieldPath {
                        dataset: dataset.name.clone(),
                        field: field.clone(),
                    }),
            );
            if let Some(condition) = &population.required_when {
                fields.extend(row_paths(&dataset.name, condition));
            }
        }
    }
    fields
}

/// The query error codes of the fields and metrics `metric`, named `name`, reads directly: the
/// codes it may hold only by propagation (E0-4.2a).
pub(super) fn upstream_codes(
    resolver: &Resolver<'_>,
    name: &MetricName,
    metric: &Metric,
) -> BTreeSet<DiagnosticCode> {
    let model = resolver.model();
    let read = reads(metric, name.second.as_ref().map(|_| &name.first));
    let population = population_fields(resolver, metric);
    let field_codes = read
        .fields
        .iter()
        .chain(&population)
        .filter_map(|path| model.dataset(&path.dataset)?.field(&path.field))
        .flat_map(|field| field.query_errors.iter().cloned());
    let metric_codes = read
        .metrics
        .iter()
        .filter_map(|input| find(resolver, input))
        .flat_map(|input| input.query_errors.iter().cloned());
    field_codes.chain(metric_codes).collect()
}

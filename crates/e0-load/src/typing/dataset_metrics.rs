//! Types the dataset-scoped metrics of the development format (section 4.5), reading their
//! references as E0.d resolves them: a bare name is a field of the metric's dataset, and
//! `source.column` a column of the catalog relation of the dataset's `source`. A dataset metric
//! reads no other metric: `AGG(…)` in it is `E0F-02` (DER-5.1.2).

use std::collections::HashMap;

use menura_model::expression::Reference;

use super::elements::{ElementTypes, column, record};
use super::output::ElementKind;
use super::scope::{Relation, Scope};
use crate::names::Name;
use crate::outline::DatasetOutline;
use crate::pointer::JsonPointer;
use menura_expressions::typing::Typed;
use menura_expressions::typing::{Aggregates, MetricAggregates, TypingScope};
use menura_expressions::typing::{Outcome, Stop};

/// The qualifier of a source column reference, compared as a regular identifier.
const SOURCE_QUALIFIER: &str = "source";

/// Types every dataset metric whose dataset's `source` the catalog describes, reading the
/// types of `fields`.
pub(crate) fn type_dataset_metrics<'model>(
    scope: &Scope<'model, '_>,
    fields: &HashMap<&JsonPointer, Typed>,
) -> ElementTypes<'model> {
    let mut found = ElementTypes::default();
    for dataset in &scope.model.datasets {
        let Some(relation) = scope.relation(dataset) else {
            continue;
        };
        let resolve = |reference: &Reference| reference_type(dataset, relation, fields, reference);
        for metric in &dataset.metrics {
            let typer = TypingScope::new(&resolve, Aggregates::Allowed)
                .with_metric_aggregates(MetricAggregates::OutsideSubset);
            record(
                scope,
                &typer,
                metric,
                ElementKind::DatasetMetric,
                &mut found,
            );
        }
    }
    found
}

/// The type of a reference of a dataset metric of `dataset`. A missing field or column, and
/// any other qualified reference (`E0F-02`, E0D-3.4), are reported by E0.d; a field without a
/// type by its own check.
fn reference_type(
    dataset: &DatasetOutline<'_>,
    relation: &Relation<'_>,
    fields: &HashMap<&JsonPointer, Typed>,
    reference: &Reference,
) -> Outcome<Typed> {
    match &reference.qualifier {
        None => dataset
            .field(&Name::of(&reference.name))
            .and_then(|field| fields.get(&field.pointer))
            .cloned()
            .ok_or(Stop::ReportedElsewhere),
        Some(qualifier) if Name::of(qualifier) == Name::regular(SOURCE_QUALIFIER) => {
            column(relation, &reference.name)
        }
        Some(_) => Err(Stop::ReportedElsewhere),
    }
}

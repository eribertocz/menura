//! What each element reads, for the propagation of unusability (E0-4.2, E0-4.2a).

use std::collections::HashMap;

use super::ModelInput;
use super::index::ModelIndex;
use super::trees::compiled;
use super::usability::read_names;
use crate::expressions::ExpressionPlace;
use crate::extensions::additivity::FieldReference;
use crate::extensions::populations::PopulationMode;
use crate::extensions::temporal::TemporalDeclaration;
use crate::names::Name;
use crate::outline::{DatasetOutline, ModelOutline, RelationshipOutline};
use crate::pointer::JsonPointer;
use crate::structure::CompiledVariant;

/// Every `(reader, read)` pair of `model`: the element at `reader` is unusable when the element
/// at `read` is.
pub(super) fn reads(
    input: &ModelInput<'_, '_>,
    model: &ModelOutline<'_>,
    index: &ModelIndex<'_, '_>,
) -> Vec<(JsonPointer, JsonPointer)> {
    let variants: HashMap<&JsonPointer, CompiledVariant<'_>> = input
        .structure
        .expressions
        .iter()
        .map(|expression| (&expression.pointer, expression.variant))
        .collect();
    let mut pairs = Vec::new();
    for dataset in &model.datasets {
        for field in &dataset.fields {
            pairs.push((field.pointer.clone(), dataset.pointer.clone()));
        }
        for metric in &dataset.metrics {
            pairs.push((metric.pointer.clone(), dataset.pointer.clone()));
            let Some(tree) = variants
                .get(&metric.pointer)
                .and_then(|variant| compiled(*variant, ExpressionPlace::Metric))
            else {
                continue;
            };
            for (qualifier, name) in read_names(&tree).references {
                if qualifier.is_none()
                    && let Some(field) = dataset.field(&name)
                {
                    pairs.push((metric.pointer.clone(), field.pointer.clone()));
                }
            }
        }
    }
    for metric in &model.metrics {
        let tree = variants
            .get(&metric.pointer)
            .and_then(|variant| compiled(*variant, ExpressionPlace::Metric));
        for read in metric_reads(input, index, &metric.pointer, tree.as_ref()) {
            pairs.push((metric.pointer.clone(), read));
        }
    }
    for relationship in &model.relationships {
        for read in relationship_reads(input, index, relationship) {
            pairs.push((relationship.pointer.clone(), read));
        }
    }
    pairs
}

/// The fields and metrics the metric at `pointer` reads: its expression, its context base and
/// time fields, and the fields of its additivity declaration.
fn metric_reads(
    input: &ModelInput<'_, '_>,
    index: &ModelIndex<'_, '_>,
    pointer: &JsonPointer,
    tree: Option<&menura_model::expression::Expression>,
) -> Vec<JsonPointer> {
    let mut read = Vec::new();
    if let Some(tree) = tree {
        let names = read_names(tree);
        for (qualifier, name) in &names.references {
            if let Some(field) = qualifier
                .as_ref()
                .and_then(|dataset| index.field(dataset, name))
            {
                read.push(field.clone());
            }
        }
        read.extend(
            names
                .metrics
                .iter()
                .filter_map(|name| index.metric(name))
                .cloned(),
        );
    }
    let mut fields: Vec<&FieldReference> = Vec::new();
    if let Some(context) = input.context.metric(pointer) {
        read.push(context.base.clone());
        fields.extend(context.modifier.time().map(|time| &time.time_dimension));
        fields.extend(context.modifier.removed_fields());
    }
    if let Some(semi) = input
        .extensions
        .additivity
        .iter()
        .find(|declaration| declaration.metric == *pointer)
        .and_then(|declaration| declaration.additivity.semi_additive())
    {
        fields.push(&semi.over);
        fields.extend(&semi.entity);
    }
    read.extend(
        fields
            .into_iter()
            .filter_map(|field| index.field(&field.dataset, &field.field))
            .cloned(),
    );
    read
}

/// The datasets a relationship joins and the fields its column lists name; for a temporal
/// relationship, also its `from_time` field and its population's `valid_from` and `valid_to`
/// fields (VAL-2.20).
fn relationship_reads(
    input: &ModelInput<'_, '_>,
    index: &ModelIndex<'_, '_>,
    relationship: &RelationshipOutline<'_>,
) -> Vec<JsonPointer> {
    let mut read = Vec::new();
    let mut ends = Vec::new();
    for (end, columns) in [
        (&relationship.from, &relationship.from_columns),
        (&relationship.to, &relationship.to_columns),
    ] {
        let dataset = end.as_ref().and_then(|end| index.dataset(&end.name));
        ends.push(dataset);
        let Some(dataset) = dataset else {
            continue;
        };
        read.push(dataset.pointer.clone());
        for column in columns {
            if let Some(field) = dataset.field(&column.name) {
                read.push(field.pointer.clone());
            }
        }
    }
    if let (Some(declaration), [from, to]) = (
        input.extensions.temporal_declaration(&relationship.pointer),
        ends.as_slice(),
    ) {
        read.extend(temporal_reads(input, *from, *to, declaration));
    }
    read
}

/// The `from_time` field of `from` and the `valid_from` and `valid_to` fields of the
/// population of `to` that `declaration` names, each when it resolves.
fn temporal_reads(
    input: &ModelInput<'_, '_>,
    from: Option<&DatasetOutline<'_>>,
    to: Option<&DatasetOutline<'_>>,
    declaration: &TemporalDeclaration,
) -> Vec<JsonPointer> {
    let mut read = Vec::new();
    if let Some(field) = from.and_then(|from| from.field(&Name::declared(&declaration.from_time))) {
        read.push(field.pointer.clone());
    }
    let Some(to) = to else {
        return read;
    };
    let wanted = Name::declared(&declaration.population);
    for population in input
        .extensions
        .populations
        .iter()
        .filter(|population| population.dataset == to.pointer)
        .filter(|population| Name::declared(&population.name) == wanted)
    {
        if let PopulationMode::Validity {
            valid_from,
            valid_to,
            ..
        } = &population.mode
        {
            for name in [valid_from, valid_to] {
                if let Some(field) = to.field(&Name::declared(name)) {
                    read.push(field.pointer.clone());
                }
            }
        }
    }
    read
}

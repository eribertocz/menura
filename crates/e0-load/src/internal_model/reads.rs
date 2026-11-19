//! What each element reads, for the propagation of unusability (E0-4.2, E0-4.2a).

use std::collections::HashMap;

use super::ModelInput;
use super::index::ModelIndex;
use super::trees::compiled;
use super::usability::read_names;
use crate::expressions::ExpressionPlace;
use crate::extensions::additivity::FieldReference;
use crate::outline::{ModelOutline, RelationshipOutline};
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
        for read in relationship_reads(index, relationship) {
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
    tree: Option<&semantic_compiler_model::expression::Expression>,
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

/// The datasets a relationship joins and the fields its column lists name.
fn relationship_reads(
    index: &ModelIndex<'_, '_>,
    relationship: &RelationshipOutline<'_>,
) -> Vec<JsonPointer> {
    let mut read = Vec::new();
    for (end, columns) in [
        (&relationship.from, &relationship.from_columns),
        (&relationship.to, &relationship.to_columns),
    ] {
        let Some(dataset) = end.as_ref().and_then(|end| index.dataset(&end.name)) else {
            continue;
        };
        read.push(dataset.pointer.clone());
        for column in columns {
            if let Some(field) = dataset.field(&column.name) {
                read.push(field.pointer.clone());
            }
        }
    }
    read
}

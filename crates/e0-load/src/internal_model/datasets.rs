//! The datasets and fields of the internal model (E0H-2 "Datasets").

use menura_model::internal_model::{
    Dataset, DatasetAccess, Field, NormalizedName, Population, RowPolicy,
};

use super::assembly::Assembly;
use super::declarations::population;
use super::located::{declared, descriptions, location_of, written};
use super::metrics::metric;
use super::security::{dataset_access, row_policy};
use crate::expressions::ExpressionPlace;
use crate::nodes::{items, member, text};
use crate::outline::{DatasetOutline, ElementOutline};
use crate::pointer::{JsonPointer, resolve};
use crate::read::{Node, Value};

/// The dataset `outline`, with its fields and dataset-scoped metrics ordered by name.
pub(super) fn dataset(assembly: &Assembly<'_, '_>, outline: &DatasetOutline<'_>) -> Dataset {
    let pointer = &outline.pointer;
    let node = resolve(assembly.document, pointer);
    Dataset {
        name: declared(outline.name.as_ref()),
        source: outline
            .source
            .as_ref()
            .map_or_else(String::new, |source| source.text.to_owned()),
        fields: by_name(
            outline
                .fields
                .iter()
                .map(|element| field(assembly, element)),
            |field| &field.name,
        ),
        metrics: by_name(
            outline
                .metrics
                .iter()
                .map(|element| metric(assembly, element)),
            |metric| &metric.name,
        ),
        primary_key: node
            .map(|node| name_list(member(node, "primary_key")))
            .unwrap_or_default(),
        unique_keys: node.map(unique_keys).unwrap_or_default(),
        access: access(assembly, pointer),
        row_policies: row_policies(assembly, pointer),
        populations: populations(assembly, pointer),
        summary: assembly.input.extensions.is_summary(pointer),
        descriptions: descriptions(assembly.document, pointer),
        query_errors: assembly.usability.codes(pointer),
        location: location_of(assembly.document, pointer),
    }
}

fn access(assembly: &Assembly<'_, '_>, pointer: &JsonPointer) -> Option<DatasetAccess> {
    assembly
        .input
        .access
        .datasets
        .iter()
        .find(|access| access.dataset == *pointer)
        .and_then(|access| access.access.as_ref())
        .map(dataset_access)
}

fn row_policies(assembly: &Assembly<'_, '_>, pointer: &JsonPointer) -> Vec<RowPolicy> {
    let policies = &assembly.input.extensions.security.policies;
    by_name(
        policies
            .iter()
            .filter(|policy| policy.dataset == *pointer)
            .map(row_policy),
        |policy| &policy.name,
    )
}

fn populations(assembly: &Assembly<'_, '_>, pointer: &JsonPointer) -> Vec<Population> {
    let declared = &assembly.input.extensions.populations;
    by_name(
        declared
            .iter()
            .filter(|population| population.dataset == *pointer)
            .map(population),
        |population| &population.name,
    )
}

/// `elements` ordered by the name `name_of` gives (E0H-1.4).
pub(super) fn by_name<Element, Name: Ord>(
    elements: impl Iterator<Item = Element>,
    name_of: impl Fn(&Element) -> &Name,
) -> Vec<Element> {
    let mut elements: Vec<Element> = elements.collect();
    elements.sort_by(|left, right| name_of(left).cmp(name_of(right)));
    elements
}

/// Each `unique_keys` entry of the dataset `node`, in the order written.
fn unique_keys(node: &Node) -> Vec<Vec<NormalizedName>> {
    items(member(node, "unique_keys"))
        .map(|(_, key)| name_list(Some(key)))
        .collect()
}

fn field(assembly: &Assembly<'_, '_>, element: &ElementOutline<'_>) -> Field {
    let pointer = &element.pointer;
    let is_time = resolve(assembly.document, pointer)
        .and_then(|node| member(node, "dimension"))
        .and_then(|dimension| member(dimension, "is_time"))
        .and_then(|value| match value.value {
            Value::Boolean(flag) => Some(flag),
            _ => None,
        });
    Field {
        name: declared(element.name.as_ref()),
        expression: assembly.tree(pointer, ExpressionPlace::Field),
        typed: assembly.typed(pointer),
        is_time,
        descriptions: descriptions(assembly.document, pointer),
        query_errors: assembly.usability.codes(pointer),
        location: location_of(assembly.document, pointer),
    }
}

/// The normalized names of a list of declared names, in the order written.
fn name_list(list: Option<&Node>) -> Vec<NormalizedName> {
    items(list)
        .filter_map(|(_, item)| text(Some(item)).map(written))
        .collect()
}

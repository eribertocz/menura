//! The names and references declared by the selected semantic model, read from the data tree
//! once so that every check works on the same located values.

use super::name::Name;
use crate::nodes::{items, member, text};
use crate::pointer::JsonPointer;
use crate::read::{Location, Node};

/// A string of the model document with where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocatedText<'document> {
    /// The string as written.
    pub(crate) text: &'document str,
    /// Its JSON Pointer.
    pub(crate) pointer: JsonPointer,
    /// Where it starts in the document text.
    pub(crate) location: Location,
}

/// A declared name or a name reference: its text and its normalized form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeclaredName<'document> {
    /// Where the name is written.
    pub(crate) written: LocatedText<'document>,
    /// The name after normalization (E0D-1.1).
    pub(crate) name: Name,
}

/// The selected semantic model, reduced to what reference resolution reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelOutline<'document> {
    /// The name of the semantic model.
    pub(crate) name: Option<DeclaredName<'document>>,
    /// The datasets, in document order.
    pub(crate) datasets: Vec<DatasetOutline<'document>>,
    /// The relationships, in document order.
    pub(crate) relationships: Vec<RelationshipOutline<'document>>,
    /// The metrics, in document order.
    pub(crate) metrics: Vec<ElementOutline<'document>>,
}

/// One dataset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DatasetOutline<'document> {
    /// Its JSON Pointer.
    pub(crate) pointer: JsonPointer,
    /// Its name.
    pub(crate) name: Option<DeclaredName<'document>>,
    /// Its `source`.
    pub(crate) source: Option<LocatedText<'document>>,
    /// Its fields, in document order.
    pub(crate) fields: Vec<ElementOutline<'document>>,
    /// The names of `primary_key`, then of every `unique_keys` entry, in document order.
    pub(crate) keys: Vec<DeclaredName<'document>>,
}

/// One field or metric: an element with a name and an expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ElementOutline<'document> {
    /// Its JSON Pointer.
    pub(crate) pointer: JsonPointer,
    /// Its name.
    pub(crate) name: Option<DeclaredName<'document>>,
    /// The JSON Pointer of its `expression`.
    pub(crate) expression_pointer: JsonPointer,
    /// Where its `expression` starts, or the element itself when it has none.
    pub(crate) expression_location: Location,
}

/// One relationship.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RelationshipOutline<'document> {
    /// Its JSON Pointer.
    pub(crate) pointer: JsonPointer,
    /// Its name.
    pub(crate) name: Option<DeclaredName<'document>>,
    /// The dataset `from` names.
    pub(crate) from: Option<DeclaredName<'document>>,
    /// The dataset `to` names.
    pub(crate) to: Option<DeclaredName<'document>>,
    /// The names of `from_columns`.
    pub(crate) from_columns: Vec<DeclaredName<'document>>,
    /// The names of `to_columns`.
    pub(crate) to_columns: Vec<DeclaredName<'document>>,
}

/// The outline of the semantic model `model`, found at `pointer`.
pub(crate) fn outline<'document>(
    model: &'document Node,
    pointer: &JsonPointer,
) -> ModelOutline<'document> {
    let datasets_pointer = pointer.member("datasets");
    let relationships_pointer = pointer.member("relationships");
    let metrics_pointer = pointer.member("metrics");
    ModelOutline {
        name: declared(model, pointer, "name"),
        datasets: items(member(model, "datasets"))
            .map(|(index, node)| dataset(node, datasets_pointer.item(index)))
            .collect(),
        relationships: items(member(model, "relationships"))
            .map(|(index, node)| relationship(node, relationships_pointer.item(index)))
            .collect(),
        metrics: items(member(model, "metrics"))
            .map(|(index, node)| element(node, metrics_pointer.item(index)))
            .collect(),
    }
}

fn dataset(node: &Node, pointer: JsonPointer) -> DatasetOutline<'_> {
    let fields_pointer = pointer.member("fields");
    let unique_keys_pointer = pointer.member("unique_keys");
    let mut keys = names(node, &pointer, "primary_key");
    for (index, key) in items(member(node, "unique_keys")) {
        keys.extend(name_list(Some(key), &unique_keys_pointer.item(index)));
    }
    DatasetOutline {
        name: declared(node, &pointer, "name"),
        source: located(node, &pointer, "source"),
        fields: items(member(node, "fields"))
            .map(|(index, field)| element(field, fields_pointer.item(index)))
            .collect(),
        keys,
        pointer,
    }
}

fn element(node: &Node, pointer: JsonPointer) -> ElementOutline<'_> {
    ElementOutline {
        name: declared(node, &pointer, "name"),
        expression_pointer: pointer.member("expression"),
        expression_location: member(node, "expression")
            .map_or(node.location, |value| value.location),
        pointer,
    }
}

fn relationship(node: &Node, pointer: JsonPointer) -> RelationshipOutline<'_> {
    RelationshipOutline {
        name: declared(node, &pointer, "name"),
        from: declared(node, &pointer, "from"),
        to: declared(node, &pointer, "to"),
        from_columns: names(node, &pointer, "from_columns"),
        to_columns: names(node, &pointer, "to_columns"),
        pointer,
    }
}

/// The string member `key` of `node`, located.
fn located<'document>(
    node: &'document Node,
    pointer: &JsonPointer,
    key: &str,
) -> Option<LocatedText<'document>> {
    let value = member(node, key)?;
    Some(LocatedText {
        text: text(Some(value))?,
        pointer: pointer.member(key),
        location: value.location,
    })
}

/// The string member `key` of `node`, as a declared name.
fn declared<'document>(
    node: &'document Node,
    pointer: &JsonPointer,
    key: &str,
) -> Option<DeclaredName<'document>> {
    located(node, pointer, key).map(into_name)
}

/// The strings of the list member `key` of `node`, as declared names.
fn names<'document>(
    node: &'document Node,
    pointer: &JsonPointer,
    key: &str,
) -> Vec<DeclaredName<'document>> {
    name_list(member(node, key), &pointer.member(key))
}

fn name_list<'document>(
    list: Option<&'document Node>,
    pointer: &JsonPointer,
) -> Vec<DeclaredName<'document>> {
    items(list)
        .filter_map(|(index, item)| {
            Some(into_name(LocatedText {
                text: text(Some(item))?,
                pointer: pointer.item(index),
                location: item.location,
            }))
        })
        .collect()
}

fn into_name(written: LocatedText<'_>) -> DeclaredName<'_> {
    DeclaredName {
        name: Name::declared(written.text),
        written,
    }
}

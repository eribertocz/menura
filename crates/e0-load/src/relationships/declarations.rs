//! The datasets and relationships of the selected semantic model, read from the data tree with
//! the names normalized, so that every check works on the same values.

use std::collections::BTreeSet;

use crate::names::Name;
use crate::nodes::{items, member, text};
use crate::pointer::JsonPointer;
use crate::read::{Location, Node};

/// A node of the model document: where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Place {
    /// Its JSON Pointer.
    pub(crate) pointer: JsonPointer,
    /// Where it starts in the document text.
    pub(crate) location: Location,
}

/// A name written in the model document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WrittenName<'document> {
    /// The name as written.
    pub(crate) text: &'document str,
    /// The name after normalization (E0D-1).
    pub(crate) name: Name,
    /// Where it is written.
    pub(crate) place: Place,
}

/// One dataset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DatasetDeclaration<'document> {
    /// Where the dataset is.
    pub(crate) place: Place,
    /// Its name.
    pub(crate) name: WrittenName<'document>,
    /// Its `primary_key`, then every `unique_keys` entry, each as the set of its names.
    pub(crate) keys: Vec<BTreeSet<Name>>,
}

/// One relationship.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RelationshipDeclaration<'document> {
    /// Where the relationship is.
    pub(crate) place: Place,
    /// Its name.
    pub(crate) name: WrittenName<'document>,
    /// The dataset `from` names.
    pub(crate) from: WrittenName<'document>,
    /// The dataset `to` names.
    pub(crate) to: WrittenName<'document>,
    /// The names of `from_columns`, in order.
    pub(crate) from_columns: Vec<Name>,
    /// The names of `to_columns`, in order.
    pub(crate) to_columns: Vec<Name>,
    /// Where `to_columns` is.
    pub(crate) to_columns_place: Place,
}

/// The datasets of the semantic model `model`, found at `pointer`, in document order. A
/// dataset without a string `name` is left out: E0.b rejects it.
pub(crate) fn datasets<'document>(
    model: &'document Node,
    pointer: &JsonPointer,
) -> Vec<DatasetDeclaration<'document>> {
    let datasets_pointer = pointer.member("datasets");
    items(member(model, "datasets"))
        .filter_map(|(index, node)| dataset(node, datasets_pointer.item(index)))
        .collect()
}

/// The relationships of the semantic model `model`, found at `pointer`, in document order. A
/// relationship without its required members is left out: E0.b rejects it.
pub(crate) fn relationships<'document>(
    model: &'document Node,
    pointer: &JsonPointer,
) -> Vec<RelationshipDeclaration<'document>> {
    let relationships_pointer = pointer.member("relationships");
    items(member(model, "relationships"))
        .filter_map(|(index, node)| relationship(node, relationships_pointer.item(index)))
        .collect()
}

fn dataset(node: &Node, pointer: JsonPointer) -> Option<DatasetDeclaration<'_>> {
    let mut keys = Vec::new();
    if let Some(primary_key) = member(node, "primary_key") {
        keys.push(name_set(primary_key));
    }
    for (_, unique_key) in items(member(node, "unique_keys")) {
        keys.push(name_set(unique_key));
    }
    Some(DatasetDeclaration {
        name: written(node, &pointer, "name")?,
        keys,
        place: Place {
            pointer,
            location: node.location,
        },
    })
}

fn relationship(node: &Node, pointer: JsonPointer) -> Option<RelationshipDeclaration<'_>> {
    let to_columns = member(node, "to_columns")?;
    Some(RelationshipDeclaration {
        name: written(node, &pointer, "name")?,
        from: written(node, &pointer, "from")?,
        to: written(node, &pointer, "to")?,
        from_columns: name_list(member(node, "from_columns")?),
        to_columns: name_list(to_columns),
        to_columns_place: Place {
            pointer: pointer.member("to_columns"),
            location: to_columns.location,
        },
        place: Place {
            pointer,
            location: node.location,
        },
    })
}

/// The string member `key` of `node`, as a written name.
fn written<'document>(
    node: &'document Node,
    pointer: &JsonPointer,
    key: &str,
) -> Option<WrittenName<'document>> {
    let value = member(node, key)?;
    let text = text(Some(value))?;
    Some(WrittenName {
        text,
        name: Name::declared(text),
        place: Place {
            pointer: pointer.member(key),
            location: value.location,
        },
    })
}

/// The names of the strings of the list `list`, in order.
fn name_list(list: &Node) -> Vec<Name> {
    items(Some(list))
        .filter_map(|(_, item)| text(Some(item)).map(Name::declared))
        .collect()
}

/// The names of the strings of the list `list`, as a set.
fn name_set(list: &Node) -> BTreeSet<Name> {
    name_list(list).into_iter().collect()
}

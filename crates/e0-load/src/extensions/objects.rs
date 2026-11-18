//! The objects of the selected semantic model that carry extension entries: `custom_extensions`
//! in the release format, `nonportable_extensions` in the development format (MDL-2.4).

use super::output::ObjectKind;
use crate::pointer::JsonPointer;
use crate::read::{Node, Value};

/// How the `data` of an entry is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DataForm {
    /// A JSON text, in a `custom_extensions` entry (MDL-2).
    Text,
    /// A YAML object, in a `nonportable_extensions` entry of the development format (MDL-2.4).
    Object,
}

/// The extension lists and the form of the `data` of their entries. Each format's schema
/// rejects the other format's list (E0B), so a model carries entries of one form only.
const LISTS: [(&str, DataForm); 2] = [
    ("custom_extensions", DataForm::Text),
    ("nonportable_extensions", DataForm::Object),
];

/// One entry of an extension list.
pub(crate) struct EntryNode<'document> {
    /// The type of the object that carries the entry.
    pub(crate) object: ObjectKind,
    /// The JSON Pointer of that object.
    pub(crate) object_pointer: JsonPointer,
    /// The JSON Pointer of the entry.
    pub(crate) pointer: JsonPointer,
    /// How its `data` is written.
    pub(crate) form: DataForm,
    /// The entry node.
    pub(crate) node: &'document Node,
}

/// Every extension entry of a structurally valid `model`, located at `pointer`, in document
/// order of the objects: the model, then each dataset followed by its fields and its
/// dataset-scoped metrics (development format), then the relationships, then the metrics.
pub(crate) fn extension_entries<'document>(
    model: &'document Node,
    pointer: &JsonPointer,
) -> Vec<EntryNode<'document>> {
    let mut entries = Vec::new();
    push_entries(&mut entries, ObjectKind::Model, model, pointer);
    let dataset_list = pointer.member("datasets");
    for (index, dataset) in items(member(model, "datasets")) {
        let dataset_pointer = dataset_list.item(index);
        push_entries(&mut entries, ObjectKind::Dataset, dataset, &dataset_pointer);
        for (name, object) in [
            ("fields", ObjectKind::Field),
            ("metrics", ObjectKind::DatasetMetric),
        ] {
            push_list(&mut entries, object, dataset, &dataset_pointer, name);
        }
    }
    for (name, object) in [
        ("relationships", ObjectKind::Relationship),
        ("metrics", ObjectKind::Metric),
    ] {
        push_list(&mut entries, object, model, pointer, name);
    }
    entries
}

/// The entries of every object of the list `name` of `parent`.
fn push_list<'document>(
    entries: &mut Vec<EntryNode<'document>>,
    object: ObjectKind,
    parent: &'document Node,
    parent_pointer: &JsonPointer,
    name: &str,
) {
    let list_pointer = parent_pointer.member(name);
    for (index, node) in items(member(parent, name)) {
        push_entries(entries, object, node, &list_pointer.item(index));
    }
}

fn push_entries<'document>(
    entries: &mut Vec<EntryNode<'document>>,
    object: ObjectKind,
    node: &'document Node,
    object_pointer: &JsonPointer,
) {
    for (list, form) in LISTS {
        let list_pointer = object_pointer.member(list);
        for (index, entry) in items(member(node, list)) {
            entries.push(EntryNode {
                object,
                object_pointer: object_pointer.clone(),
                pointer: list_pointer.item(index),
                form,
                node: entry,
            });
        }
    }
}

/// The value of the member `name` of a mapping node.
pub(crate) fn member<'node>(node: &'node Node, name: &str) -> Option<&'node Node> {
    let Value::Mapping(entries) = &node.value else {
        return None;
    };
    entries
        .iter()
        .find(|entry| matches!(&entry.key.value, Value::String(key) if key == name))
        .map(|entry| &entry.value)
}

/// The items of a sequence node with their indices; nothing for any other node.
fn items(node: Option<&Node>) -> impl Iterator<Item = (usize, &Node)> {
    let items: &[Node] = match node.map(|node| &node.value) {
        Some(Value::Sequence(items)) => items,
        _ => &[],
    };
    items.iter().enumerate()
}

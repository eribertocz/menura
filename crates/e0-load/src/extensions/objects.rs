//! The objects of the selected semantic model that carry `custom_extensions`.

use super::output::ObjectKind;
use crate::pointer::JsonPointer;
use crate::read::{Node, Value};

/// One entry of a `custom_extensions` list.
pub(crate) struct EntryNode<'document> {
    /// The type of the object that carries the entry.
    pub(crate) object: ObjectKind,
    /// The JSON Pointer of that object.
    pub(crate) object_pointer: JsonPointer,
    /// The JSON Pointer of the entry.
    pub(crate) pointer: JsonPointer,
    /// The entry node.
    pub(crate) node: &'document Node,
}

/// Every `custom_extensions` entry of a structurally valid `model`, located at `pointer`, in
/// document order of the objects: the model, then each dataset followed by its fields, then
/// the relationships, then the metrics.
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
        let field_list = dataset_pointer.member("fields");
        for (field_index, field) in items(member(dataset, "fields")) {
            let field_pointer = field_list.item(field_index);
            push_entries(&mut entries, ObjectKind::Field, field, &field_pointer);
        }
    }
    for (name, object) in [
        ("relationships", ObjectKind::Relationship),
        ("metrics", ObjectKind::Metric),
    ] {
        let list_pointer = pointer.member(name);
        for (index, node) in items(member(model, name)) {
            push_entries(&mut entries, object, node, &list_pointer.item(index));
        }
    }
    entries
}

fn push_entries<'document>(
    entries: &mut Vec<EntryNode<'document>>,
    object: ObjectKind,
    node: &'document Node,
    object_pointer: &JsonPointer,
) {
    let list_pointer = object_pointer.member("custom_extensions");
    for (index, entry) in items(member(node, "custom_extensions")) {
        entries.push(EntryNode {
            object,
            object_pointer: object_pointer.clone(),
            pointer: list_pointer.item(index),
            node: entry,
        });
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

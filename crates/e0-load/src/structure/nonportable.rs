//! The development-line `nonportable_extensions` entries of the selected model (MDL-2.4).

use super::output::{ExtensionOwner, NonportableExtension};
use crate::nodes::{items, member, text};
use crate::pointer::JsonPointer;
use crate::read::Node;

/// Every `nonportable_extensions` entry of a structurally valid `model`, located at `pointer`,
/// in the order of their objects: the model, then each dataset followed by its fields and its
/// dataset-scoped metrics, then the relationships, then the metrics. The caller sorts them.
pub(crate) fn nonportable_extensions<'document>(
    model: &'document Node,
    pointer: &JsonPointer,
) -> Vec<NonportableExtension<'document>> {
    let mut entries = Vec::new();
    push_entries(&mut entries, ExtensionOwner::Model, model, pointer);
    let dataset_list = pointer.member("datasets");
    for (index, dataset) in items(member(model, "datasets")) {
        let dataset_pointer = dataset_list.item(index);
        push_entries(
            &mut entries,
            ExtensionOwner::Dataset,
            dataset,
            &dataset_pointer,
        );
        for (list, owner) in [
            ("fields", ExtensionOwner::Field),
            ("metrics", ExtensionOwner::DatasetMetric),
        ] {
            push_list(&mut entries, owner, dataset, &dataset_pointer, list);
        }
    }
    for (list, owner) in [
        ("relationships", ExtensionOwner::Relationship),
        ("metrics", ExtensionOwner::Metric),
    ] {
        push_list(&mut entries, owner, model, pointer, list);
    }
    entries
}

/// The entries of every object of the list `list` of `parent`.
fn push_list<'document>(
    entries: &mut Vec<NonportableExtension<'document>>,
    owner: ExtensionOwner,
    parent: &'document Node,
    parent_pointer: &JsonPointer,
    list: &str,
) {
    let list_pointer = parent_pointer.member(list);
    for (index, object) in items(member(parent, list)) {
        push_entries(entries, owner, object, &list_pointer.item(index));
    }
}

/// The entries of one object.
fn push_entries<'document>(
    entries: &mut Vec<NonportableExtension<'document>>,
    owner: ExtensionOwner,
    object: &'document Node,
    owner_pointer: &JsonPointer,
) {
    let list_pointer = owner_pointer.member("nonportable_extensions");
    for (index, entry) in items(member(object, "nonportable_extensions")) {
        let (Some(vendor_name), Some(data)) =
            (text(member(entry, "vendor_name")), member(entry, "data"))
        else {
            continue;
        };
        entries.push(NonportableExtension {
            owner,
            owner_pointer: owner_pointer.clone(),
            pointer: list_pointer.item(index),
            vendor_name,
            data,
        });
    }
}

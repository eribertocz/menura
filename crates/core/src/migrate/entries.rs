//! The extension entries of every model of a document: the entries of `custom_extensions` in
//! the release format and of `nonportable_extensions` in the development format (MDL-2.4), on
//! the objects that carry them (E0C-1): each model, its datasets, their fields and
//! dataset-scoped metrics, its relationships and its metrics.

use menura_contracts::types::audit_record::OssieFormat;
use menura_contracts::types::ordered_json::{OrderedJson, OrderedObject};

use super::location::Location;

/// Every extension entry of the document `tree` in `format`, with its location.
pub(super) fn extension_entries(
    tree: &mut OrderedObject,
    format: OssieFormat,
) -> Vec<(&mut OrderedJson, Location)> {
    let mut entries = Vec::new();
    let root = Location::default();
    match format {
        OssieFormat::Release => {
            for (model, location) in items(tree, "semantic_model", &root) {
                if let OrderedJson::Object(model) = model {
                    model_entries(model, &location, format, &mut entries);
                }
            }
        }
        OssieFormat::Development => model_entries(tree, &root, format, &mut entries),
    }
    entries
}

/// The entries of the semantic model `model` at `location` and of its objects.
fn model_entries<'tree>(
    model: &'tree mut OrderedObject,
    location: &Location,
    format: OssieFormat,
    entries: &mut Vec<(&'tree mut OrderedJson, Location)>,
) {
    let list = list_name(format);
    for (name, value) in member_locations(model, location) {
        match name {
            "datasets" => {
                for (dataset, dataset_location) in array_items(value) {
                    if let OrderedJson::Object(dataset) = dataset {
                        dataset_entries(dataset, &dataset_location, list, entries);
                    }
                }
            }
            "relationships" | "metrics" => object_list_entries(value, list, entries),
            _ if name == list => entries.extend(array_items(value)),
            _ => {}
        }
    }
}

/// The entries of the dataset `dataset` at `location`, of its fields and of its metrics.
fn dataset_entries<'tree>(
    dataset: &'tree mut OrderedObject,
    location: &Location,
    list: &str,
    entries: &mut Vec<(&'tree mut OrderedJson, Location)>,
) {
    for (name, value) in member_locations(dataset, location) {
        match name {
            "fields" | "metrics" => object_list_entries(value, list, entries),
            _ if name == list => entries.extend(array_items(value)),
            _ => {}
        }
    }
}

/// The entries of every object of the array `value`.
fn object_list_entries<'tree>(
    value: (&'tree mut OrderedJson, Location),
    list: &str,
    entries: &mut Vec<(&'tree mut OrderedJson, Location)>,
) {
    for (object, object_location) in array_items(value) {
        if let OrderedJson::Object(object) = object {
            for (name, value) in member_locations(object, &object_location) {
                if name == list {
                    entries.extend(array_items(value));
                }
            }
        }
    }
}

/// The name of the extension list of `format` (MDL-2.4).
fn list_name(format: OssieFormat) -> &'static str {
    match format {
        OssieFormat::Release => "custom_extensions",
        OssieFormat::Development => "nonportable_extensions",
    }
}

/// Every member of `object` at `location`, with its name and location.
fn member_locations<'tree>(
    object: &'tree mut OrderedObject,
    location: &Location,
) -> impl Iterator<Item = (&'tree str, (&'tree mut OrderedJson, Location))> {
    object
        .members_mut()
        .iter_mut()
        .enumerate()
        .map(|(index, (name, value))| {
            let member = location.member(name, index);
            (name.as_str(), (value, member))
        })
}

/// The items of the member `name` of `object` at `location`, when it is an array.
fn items<'tree>(
    object: &'tree mut OrderedObject,
    name: &str,
    location: &Location,
) -> Vec<(&'tree mut OrderedJson, Location)> {
    member_locations(object, location)
        .find(|(member, _)| *member == name)
        .map(|(_, value)| array_items(value))
        .unwrap_or_default()
}

/// The items of `value` with their locations, when it is an array; nothing otherwise.
fn array_items(
    (value, location): (&mut OrderedJson, Location),
) -> Vec<(&mut OrderedJson, Location)> {
    match value {
        OrderedJson::Array(items) => items
            .iter_mut()
            .enumerate()
            .map(|(index, item)| (item, location.item(index)))
            .collect(),
        _ => Vec::new(),
    }
}

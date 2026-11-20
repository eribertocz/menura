//! Which extension entries are ours (E0C-1, MDL-2.4, MDL-2.5).

use menura_contracts::FORMER_NAMESPACE;
use serde_json::{Map, Value as Json};

use super::diagnostic::{ExtensionCode, ExtensionDiagnostic};
use super::keys::NAMESPACE;
use super::members::read_members;
use super::object_data::json_value;
use super::objects::{DataForm, EntryNode, member};
use crate::pointer::JsonPointer;
use crate::read::{Node, Value};

/// The vendor of the entries that may be ours (E0C-1, condition 1).
const COMMON: &str = "COMMON";

/// What one extension entry is.
pub(crate) enum EntryKind<'document> {
    /// Someone else's entry: it is kept and never interpreted (E0C-1.1).
    Foreign,
    /// A `COMMON` entry whose `data` is not valid JSON: kept, not interpreted, and reported as
    /// `W0C-01` (E0C-1.2).
    NotJson(ExtensionDiagnostic),
    /// An entry of ours.
    Ours(OwnData<'document>),
}

/// The `data` of an entry of ours.
pub(crate) struct OwnData<'document> {
    /// The JSON Pointer of the `data` member.
    pub(crate) pointer: JsonPointer,
    /// The `data` node.
    pub(crate) node: &'document Node,
    /// The members of the JSON object `data` holds. When a member name is repeated, the last
    /// value is kept, as the JSON reader does, and [`Self::repeated_member`] names it.
    pub(crate) members: Map<String, Json>,
    /// The first member name repeated in an object anywhere inside `data` (E0C-2.7).
    pub(crate) repeated_member: Option<String>,
    /// Why the object `data` of a development-line entry has no JSON form (E0C-2.8); the
    /// members are then empty.
    pub(crate) without_json_form: Option<String>,
    /// Whether the entry is ours only through the former namespace, which reports `W0C-02`
    /// (CON-0.5).
    pub(crate) former_namespace: bool,
}

/// How the `namespace` values of an entry make it ours (E0C-1, condition 3).
#[derive(Clone, Copy)]
enum NamespaceMatch {
    /// A value equals the configured namespace.
    Configured,
    /// No value equals the configured namespace, but one equals the former namespace
    /// `semantic_compiler`, accepted until 1.0.0 (CON-0.5).
    Former,
}

/// How `values` make an entry ours for the configured `namespace`, or `None` when they do not.
/// When `namespace` is the former namespace itself, the entry matches it as configured.
fn namespace_match(values: &[Option<&str>], namespace: &str) -> Option<NamespaceMatch> {
    let any_equals = |wanted: &str| values.contains(&Some(wanted));
    if any_equals(namespace) {
        Some(NamespaceMatch::Configured)
    } else if any_equals(FORMER_NAMESPACE) {
        Some(NamespaceMatch::Former)
    } else {
        None
    }
}

/// What `entry` is, for the configured `namespace` (E0C-1, MDL-2.3).
///
/// An entry is ours only when its vendor is `COMMON`, its `data` is a JSON object and
/// `data.namespace` equals `namespace` or, until 1.0.0, the former namespace `semantic_compiler`
/// (CON-0.5). A JSON `data` that is not an object has no namespace,
/// so it is someone else's. When `namespace` is repeated in `data`, the entry is ours if any of
/// its values equals `namespace` (E0C-1.3): it then has a reading as ours, and the repeated
/// member rejects it (E0C-2.7).
pub(crate) fn classify<'document>(
    entry: &EntryNode<'document>,
    namespace: &str,
) -> EntryKind<'document> {
    if !is_text(member(entry.node, "vendor_name"), COMMON) {
        return EntryKind::Foreign;
    }
    let Some(node) = member(entry.node, "data") else {
        return EntryKind::Foreign;
    };
    let pointer = entry.pointer.member("data");
    match entry.form {
        DataForm::Text => classify_text(node, pointer, namespace),
        DataForm::Object => classify_object(node, pointer, namespace),
    }
}

/// What a `COMMON` entry whose `data` is a JSON text is (E0C-1).
fn classify_text<'document>(
    node: &'document Node,
    pointer: JsonPointer,
    namespace: &str,
) -> EntryKind<'document> {
    let Value::String(text) = &node.value else {
        return EntryKind::Foreign;
    };
    let parsed = match read_members(text) {
        Ok(parsed) => parsed,
        Err(error) => {
            return EntryKind::NotJson(ExtensionDiagnostic {
                code: ExtensionCode::NotJson,
                pointer,
                location: node.location,
                message: format!(
                    "the COMMON entry is kept and not interpreted: its data is not valid JSON ({error})"
                ),
            });
        }
    };
    let Json::Object(members) = parsed.value else {
        return EntryKind::Foreign;
    };
    let values: Vec<Option<&str>> = parsed.namespaces.iter().map(Json::as_str).collect();
    let Some(matched) = namespace_match(&values, namespace) else {
        return EntryKind::Foreign;
    };
    EntryKind::Ours(OwnData {
        pointer,
        node,
        members,
        repeated_member: parsed.repeated,
        without_json_form: None,
        former_namespace: matches!(matched, NamespaceMatch::Former),
    })
}

/// What a `COMMON` entry of the development format, whose `data` is a YAML object, is
/// (MDL-2.4). `data` is read as the JSON value it represents (E0C-2.8). A YAML key is never
/// repeated (E0A-2.3), so `namespace` has one value and no member is repeated.
fn classify_object<'document>(
    node: &'document Node,
    pointer: JsonPointer,
    namespace: &str,
) -> EntryKind<'document> {
    let value = member(node, NAMESPACE).and_then(|node| match &node.value {
        Value::String(text) => Some(text.as_str()),
        _ => None,
    });
    let Some(matched) = namespace_match(&[value], namespace) else {
        return EntryKind::Foreign;
    };
    let (members, without_json_form) = match json_value(node) {
        Ok(Json::Object(members)) => (members, None),
        Ok(_) => return EntryKind::Foreign,
        Err(reason) => (Map::new(), Some(reason)),
    };
    EntryKind::Ours(OwnData {
        pointer,
        node,
        members,
        repeated_member: None,
        without_json_form,
        former_namespace: matches!(matched, NamespaceMatch::Former),
    })
}

fn is_text(node: Option<&Node>, text: &str) -> bool {
    matches!(node.map(|node| &node.value), Some(Value::String(value)) if value == text)
}

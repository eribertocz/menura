//! Which `custom_extensions` entries are ours (E0C-1, MDL-2.5).

use serde_json::{Map, Value as Json};

use super::diagnostic::{ExtensionCode, ExtensionDiagnostic};
use super::members::read_members;
use super::objects::{EntryNode, member};
use crate::pointer::JsonPointer;
use crate::read::{Node, Value};

/// The vendor of the entries that may be ours (E0C-1, condition 1).
const COMMON: &str = "COMMON";

/// What one `custom_extensions` entry is.
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
}

/// What `entry` is, for the configured `namespace` (E0C-1, MDL-2.3).
///
/// An entry is ours only when its vendor is `COMMON`, its `data` is a JSON object and
/// `data.namespace` equals `namespace`. A JSON `data` that is not an object has no namespace,
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
    let Value::String(text) = &node.value else {
        return EntryKind::Foreign;
    };
    let pointer = entry.pointer.member("data");
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
    if !parsed
        .namespaces
        .iter()
        .any(|value| value.as_str() == Some(namespace))
    {
        return EntryKind::Foreign;
    }
    EntryKind::Ours(OwnData {
        pointer,
        node,
        members,
        repeated_member: parsed.repeated,
    })
}

fn is_text(node: Option<&Node>, text: &str) -> bool {
    matches!(node.map(|node| &node.value), Some(Value::String(value)) if value == text)
}

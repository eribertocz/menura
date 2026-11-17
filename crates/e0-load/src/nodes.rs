//! Read access to the nodes of the located data tree, shared by the substages that walk it.

use crate::read::{Node, Value};

/// The value of the member `name` of a mapping node; `None` for any other node.
pub(crate) fn member<'node>(node: &'node Node, name: &str) -> Option<&'node Node> {
    let Value::Mapping(entries) = &node.value else {
        return None;
    };
    entries
        .iter()
        .find(|entry| is_text(&entry.key, name))
        .map(|entry| &entry.value)
}

/// The items of a sequence node with their indices; nothing for any other node.
pub(crate) fn items(node: Option<&Node>) -> impl Iterator<Item = (usize, &Node)> {
    let items: &[Node] = match node.map(|node| &node.value) {
        Some(Value::Sequence(items)) => items,
        _ => &[],
    };
    items.iter().enumerate()
}

/// The text of a string node; `None` for any other node.
pub(crate) fn text(node: Option<&Node>) -> Option<&str> {
    match node.map(|node| &node.value) {
        Some(Value::String(text)) => Some(text),
        _ => None,
    }
}

/// Whether `node` is the string `text`.
pub(crate) fn is_text(node: &Node, text: &str) -> bool {
    matches!(&node.value, Value::String(value) if value == text)
}

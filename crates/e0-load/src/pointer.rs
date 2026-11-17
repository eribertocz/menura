//! JSON Pointers (RFC 6901) into the located data tree, the way E0 diagnostics locate a node
//! (E0B-3.6), and the document order of E0-4.3.

use std::fmt;

use crate::read::{Node, Value};

/// A JSON Pointer (RFC 6901) to a node of the model document, such as
/// `/semantic_model/0/datasets/2/fields/1/expression`. The empty pointer is the whole document.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct JsonPointer(String);

impl JsonPointer {
    /// The pointer to the whole document.
    #[must_use]
    pub fn root() -> Self {
        Self::default()
    }

    /// The pointer to the member `name` of the object this pointer names. `~` and `/` in the
    /// name are escaped as RFC 6901 §3 requires.
    #[must_use]
    pub fn member(&self, name: &str) -> Self {
        let escaped = name.replace('~', "~0").replace('/', "~1");
        Self(format!("{}/{escaped}", self.0))
    }

    /// The pointer to the item `index` of the array this pointer names.
    #[must_use]
    pub fn item(&self, index: usize) -> Self {
        Self(format!("{}/{index}", self.0))
    }

    /// The pointer as RFC 6901 writes it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// A pointer from its RFC 6901 text, as a JSON Schema evaluator reports it.
    pub(crate) fn from_text(text: &str) -> Self {
        Self(text.to_owned())
    }

    /// The unescaped reference tokens, in order.
    fn tokens(&self) -> impl Iterator<Item = String> + '_ {
        self.0
            .split('/')
            .skip(1)
            .map(|token| token.replace("~1", "/").replace("~0", "~"))
    }
}

impl fmt::Display for JsonPointer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The text of a mapping key, as it appears in a JSON Pointer and in the JSON form of the
/// document. YAML allows keys that are not strings; they are written as their JSON text.
pub(crate) fn key_text(key: &Node) -> String {
    match &key.value {
        Value::String(text) => text.clone(),
        Value::Null => "null".to_owned(),
        Value::Boolean(value) => value.to_string(),
        Value::Integer(value) => value.to_string(),
        Value::Float(value) => value.to_string(),
        Value::Sequence(_) | Value::Mapping(_) => String::new(),
    }
}

/// The position of the node `pointer` names: for each token, the index of the entry or item it
/// selects. Comparing positions compares nodes in document order, a node before its children
/// (E0-4.3). Returns `None` when the pointer names no node.
pub(crate) fn position(root: &Node, pointer: &JsonPointer) -> Option<Vec<usize>> {
    let mut node = root;
    let mut indices = Vec::new();
    for token in pointer.tokens() {
        let (index, child) = step(node, &token)?;
        indices.push(index);
        node = child;
    }
    Some(indices)
}

/// The node `pointer` names, if any.
pub(crate) fn resolve<'node>(root: &'node Node, pointer: &JsonPointer) -> Option<&'node Node> {
    pointer.tokens().try_fold(root, |node, token| {
        step(node, &token).map(|(_, child)| child)
    })
}

/// The child of `node` that `token` selects, with its index. When two keys of a mapping have
/// the same text, the first one is selected, as in the JSON form of the document.
fn step<'node>(node: &'node Node, token: &str) -> Option<(usize, &'node Node)> {
    match &node.value {
        Value::Mapping(entries) => entries
            .iter()
            .enumerate()
            .find(|(_, entry)| key_text(&entry.key) == token)
            .map(|(index, entry)| (index, &entry.value)),
        Value::Sequence(items) => {
            let index: usize = token.parse().ok()?;
            items.get(index).map(|item| (index, item))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{JsonPointer, position, resolve};
    use crate::read::{Value, read_document};

    #[test]
    fn e0b_3_6_pointers_escape_and_resolve() {
        let document = read_document(b"b: 1\na/b~c: [x, y]\n").expect("the document is read");
        let pointer = JsonPointer::root().member("a/b~c").item(1);
        assert_eq!(pointer.as_str(), "/a~1b~0c/1");
        let node = resolve(&document, &pointer).expect("the pointer names a node");
        assert_eq!(node.value, Value::String("y".to_owned()));
        assert_eq!(position(&document, &pointer), Some(vec![1, 1]));
        assert_eq!(position(&document, &JsonPointer::root().member("c")), None);
    }

    #[test]
    fn e0_4_3_positions_follow_document_order() {
        let document = read_document(b"z: {b: 1, a: 2}\na: 3\n").expect("the document is read");
        let at = |pointer: &str| position(&document, &JsonPointer::from_text(pointer));
        assert!(at("/z") < at("/z/b"));
        assert!(at("/z/b") < at("/z/a"));
        assert!(at("/z/a") < at("/a"));
    }
}

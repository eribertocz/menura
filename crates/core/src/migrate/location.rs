//! Where a node of the model tree is: its RFC 6901 pointer, and its position, which orders
//! nodes in document order (MIG-3.4).

/// The location of a node of the model tree.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Location {
    /// The RFC 6901 pointer of the node.
    pub(super) pointer: String,
    /// For each step from the root, the index of the member or item it selects. Comparing
    /// positions compares nodes in document order.
    pub(super) position: Vec<usize>,
}

impl Location {
    /// The location of the member `name`, the `index`-th member of this object.
    pub(super) fn member(&self, name: &str, index: usize) -> Self {
        let token = name.replace('~', "~0").replace('/', "~1");
        self.step(&token, index)
    }

    /// The location of the item `index` of this array.
    pub(super) fn item(&self, index: usize) -> Self {
        self.step(&index.to_string(), index)
    }

    fn step(&self, token: &str, index: usize) -> Self {
        let mut position = self.position.clone();
        position.push(index);
        Self {
            pointer: format!("{}/{token}", self.pointer),
            position,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Location;

    #[test]
    fn pointer_tokens_are_escaped() {
        let location = Location::default().member("a/b~c", 3).item(0);
        assert_eq!(location.pointer, "/a~1b~0c/0");
        assert_eq!(location.position, vec![3, 0]);
    }
}

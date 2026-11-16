//! Construction of the located data tree from the raw tree: tags, aliases, merge keys and
//! duplicate keys (E0A-2.2 to E0A-2.5, E0A-2.7).
//!
//! An anchored node is constructed once; every alias to it shares the result. A mapping first
//! takes the entries of its merge key `<<` (a mapping, or a sequence of mappings where an earlier
//! mapping wins over a later one), then its own entries, which win over merged ones. A key keeps
//! the position of its first insertion. Repeating a key among a mapping's own entries, the merge
//! key included, is an error (DEV-1).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::compose::{RawContent, RawNode, RawTag};
use super::error::{ReadError, ReadErrorCode};
use super::key::{KeyIdentity, key_identity};
use super::scalar::{ScalarPosition, construct_scalar};
use super::tree::{Entry, Location, Node, Value};

/// Constructs the data tree of a raw tree.
///
/// # Errors
///
/// `E0A-04` for a repeated key, `E0A-05` for a tag or value that cannot be constructed; the first
/// one in document order.
pub(crate) fn construct(root: &RawNode) -> Result<Node, ReadError> {
    Constructor::default().node(root, ScalarPosition::Value)
}

#[derive(Default)]
struct Constructor {
    /// The constructed anchored nodes, by anchor identifier.
    anchored: BTreeMap<usize, Node>,
}

impl Constructor {
    fn node(&mut self, raw: &RawNode, position: ScalarPosition) -> Result<Node, ReadError> {
        let value = match &raw.content {
            RawContent::Alias(anchor) => return self.alias(*anchor, raw.location),
            RawContent::Scalar { text, plain } => {
                construct_scalar(text, *plain, &raw.tag, position)
                    .map_err(|message| ReadError::unconstructible(raw.tag_location, message))?
            }
            RawContent::Sequence(items) => {
                check_collection_tag(raw, "seq")?;
                let items = items
                    .iter()
                    .map(|item| self.node(item, ScalarPosition::Value))
                    .collect::<Result<Vec<_>, _>>()?;
                Value::Sequence(Arc::from(items))
            }
            RawContent::Mapping(entries) => {
                check_collection_tag(raw, "map")?;
                Value::Mapping(Arc::from(self.mapping(entries)?))
            }
        };
        let node = Node {
            value,
            location: raw.location,
        };
        if raw.anchor != 0 {
            self.anchored.insert(raw.anchor, node.clone());
        }
        Ok(node)
    }

    /// An alias whose anchor is not constructed yet sits inside its own anchored node.
    fn alias(&self, anchor: usize, location: Location) -> Result<Node, ReadError> {
        self.anchored.get(&anchor).cloned().ok_or_else(|| {
            ReadError::unconstructible(location, "the alias refers to a node that contains it")
        })
    }

    fn mapping(&mut self, raw_entries: &[(RawNode, RawNode)]) -> Result<Vec<Entry>, ReadError> {
        let mut merged = Vec::new();
        let mut merge_key_seen = false;
        let mut own = Vec::with_capacity(raw_entries.len());
        let mut own_keys = BTreeSet::new();
        for (raw_key, raw_value) in raw_entries {
            if is_merge_key(raw_key) {
                if merge_key_seen {
                    return Err(duplicate_key(raw_key.location));
                }
                merge_key_seen = true;
                merged = self.merged_entries(raw_value)?;
                continue;
            }
            let key = self.key(raw_key)?;
            if let Some(identity) = key_identity(&key.value)
                && !own_keys.insert(identity)
            {
                return Err(duplicate_key(raw_key.location));
            }
            let value = self.node(raw_value, ScalarPosition::Value)?;
            own.push(Entry { key, value });
        }
        Ok(combine(merged.into_iter().chain(own)))
    }

    /// A key is a scalar, or an alias to one.
    fn key(&mut self, raw: &RawNode) -> Result<Node, ReadError> {
        let key = self.node(raw, ScalarPosition::Key)?;
        if matches!(key.value, Value::Sequence(_) | Value::Mapping(_)) {
            return Err(ReadError::unconstructible(
                raw.location,
                "a mapping key must be a scalar",
            ));
        }
        Ok(key)
    }

    /// The entries a merge key brings, in the order that makes the earlier mapping win.
    fn merged_entries(&mut self, raw: &RawNode) -> Result<Vec<Entry>, ReadError> {
        let source = self.node(raw, ScalarPosition::Value)?;
        match &source.value {
            Value::Mapping(entries) => Ok(entries.to_vec()),
            Value::Sequence(items) => {
                let mut entries = Vec::new();
                for item in items.iter().rev() {
                    let Value::Mapping(item_entries) = &item.value else {
                        return Err(not_mergeable(item.location));
                    };
                    entries.extend(item_entries.iter().cloned());
                }
                Ok(entries)
            }
            _ => Err(not_mergeable(source.location)),
        }
    }
}

/// Inserts the entries in order; a later entry with an equal key replaces the value of the
/// earlier one and keeps its position.
fn combine(entries: impl Iterator<Item = Entry>) -> Vec<Entry> {
    let mut combined: Vec<Entry> = Vec::new();
    let mut positions: BTreeMap<KeyIdentity, usize> = BTreeMap::new();
    for entry in entries {
        let Some(identity) = key_identity(&entry.key.value) else {
            combined.push(entry);
            continue;
        };
        if let Some(existing) = positions
            .get(&identity)
            .and_then(|&index| combined.get_mut(index))
        {
            existing.value = entry.value;
        } else {
            positions.insert(identity, combined.len());
            combined.push(entry);
        }
    }
    combined
}

/// The merge key is the plain scalar `<<`, or a key tagged `!!merge`.
fn is_merge_key(raw: &RawNode) -> bool {
    match (&raw.content, &raw.tag) {
        (RawContent::Scalar { text, plain: true }, RawTag::Absent) => text == "<<",
        (RawContent::Scalar { .. }, RawTag::Standard(suffix)) => suffix == "merge",
        _ => false,
    }
}

/// A sequence takes no tag but `!` and `!!seq`; a mapping none but `!` and `!!map`.
fn check_collection_tag(raw: &RawNode, expected: &str) -> Result<(), ReadError> {
    match &raw.tag {
        RawTag::Absent | RawTag::NonSpecific => Ok(()),
        RawTag::Standard(suffix) if suffix == expected => Ok(()),
        RawTag::Standard(suffix) => Err(ReadError::unconstructible(
            raw.tag_location,
            format!("no constructor for the tag !!{suffix} on this node"),
        )),
        RawTag::Other(name) => Err(ReadError::unconstructible(
            raw.tag_location,
            format!("no constructor for the tag '{name}'"),
        )),
    }
}

fn duplicate_key(location: Location) -> ReadError {
    ReadError::new(
        ReadErrorCode::DuplicateKey,
        location,
        "the mapping repeats this key",
    )
}

fn not_mergeable(location: Location) -> ReadError {
    ReadError::unconstructible(
        location,
        "a merge key takes a mapping or a sequence of mappings",
    )
}

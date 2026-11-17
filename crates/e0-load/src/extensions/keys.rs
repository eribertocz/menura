//! The extension keys and the object type each one is valid on (MDL-2, E0C-2.3).

use super::output::ObjectKind;

/// The member of `data` that names the namespace (E0C-1).
pub(crate) const NAMESPACE: &str = "namespace";

/// The member of `data` that names the extension version (E0C-2.1).
pub(crate) const VERSION: &str = "version";

/// The only supported extension version (E0C-2.1).
pub(crate) const SUPPORTED_VERSION: u64 = 1;

/// Every extension key with the object type it is valid on, as the model contract lists them.
const EXTENSION_KEYS: [(&str, ObjectKind); 8] = [
    ("additivity", ObjectKind::Metric),
    ("populations", ObjectKind::Dataset),
    ("derived", ObjectKind::Metric),
    ("context", ObjectKind::Metric),
    ("security_context", ObjectKind::Model),
    ("access", ObjectKind::Dataset),
    ("row_access", ObjectKind::Dataset),
    ("aggregate", ObjectKind::Dataset),
];

/// What a key of `data` of an entry of ours is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyKind {
    /// `namespace` or `version`.
    Envelope,
    /// An extension key, valid only on the given object type.
    Extension(ObjectKind),
    /// A key that no extension defines.
    Unknown,
}

/// What `key` is.
pub(crate) fn key_kind(key: &str) -> KeyKind {
    if key == NAMESPACE || key == VERSION {
        return KeyKind::Envelope;
    }
    EXTENSION_KEYS
        .iter()
        .find(|(name, _)| *name == key)
        .map_or(KeyKind::Unknown, |(_, object)| KeyKind::Extension(*object))
}

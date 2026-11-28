//! The extension keys and the object types each one is valid on (MDL-2, E0C-2.3).

use super::output::ObjectKind;

/// The member of `data` that names the namespace (E0C-1).
pub(crate) const NAMESPACE: &str = "namespace";

/// The member of `data` that names the extension version (E0C-2.1).
pub(crate) const VERSION: &str = "version";

/// The only supported extension version (E0C-2.1).
pub(crate) const SUPPORTED_VERSION: u64 = 1;

/// Every extension key with the object types it is valid on, as the model contract lists them.
/// `additivity` is valid on a dataset metric of the development format too (ADD-5.1.6);
/// `derived` and `context` are valid on model metrics only (DER-5.1.1, CTX-5.1.1), and
/// `temporal` on relationships only (VAL-2.1).
const EXTENSION_KEYS: [(&str, &[ObjectKind]); 9] = [
    (
        "additivity",
        &[ObjectKind::Metric, ObjectKind::DatasetMetric],
    ),
    ("populations", &[ObjectKind::Dataset]),
    ("derived", &[ObjectKind::Metric]),
    ("context", &[ObjectKind::Metric]),
    ("security_context", &[ObjectKind::Model]),
    ("access", &[ObjectKind::Dataset]),
    ("row_access", &[ObjectKind::Dataset]),
    ("aggregate", &[ObjectKind::Dataset]),
    ("temporal", &[ObjectKind::Relationship]),
];

/// What a key of `data` of an entry of ours is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyKind {
    /// `namespace` or `version`.
    Envelope,
    /// An extension key, valid only on the given object types.
    Extension(&'static [ObjectKind]),
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
        .map_or(KeyKind::Unknown, |(_, objects)| KeyKind::Extension(objects))
}

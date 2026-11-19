//! The integrity checks the model needs, recorded as data (E0H-4.3, CHK-2, CHK-3).

use semantic_compiler_contracts::Sha256Hash;

/// The `kind` of an integrity check (CHK-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CheckKind {
    /// `key_uniqueness`: `to_columns` are unique in `to`.
    KeyUniqueness,
    /// `orphan_keys`: every `from_columns` value matches `to`.
    OrphanKeys,
    /// `summary`: a summary matches its base.
    Summary,
    /// `population`: the expected entities per period.
    Population,
    /// `semi_additive`: the snapshots of a semi-additive metric.
    SemiAdditive,
}

impl CheckKind {
    /// The kind as CHK-2 writes it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::KeyUniqueness => "key_uniqueness",
            Self::OrphanKeys => "orphan_keys",
            Self::Summary => "summary",
            Self::Population => "population",
            Self::SemiAdditive => "semi_additive",
        }
    }
}

/// One recorded integrity check; its SQL is generated later (CHK-4.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedCheck {
    /// `<kind>:<target>` (CHK-2).
    pub check_id: String,
    /// The kind.
    pub kind: CheckKind,
    /// The normalized name of the relationship, summary, population or metric; a
    /// development-line dataset metric is qualified as `<dataset>.<metric>` (E0H-4.3).
    pub target: String,
    /// The definition hash (CHK-3).
    pub definition_hash: Sha256Hash,
}

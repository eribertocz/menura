//! A reference of the wide-table form, with every part as written (E1-3.1).

/// One part of a reference, between `.` separators outside delimited identifiers (E1-3.1).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ReferencePart {
    /// A regular identifier, as written; it may be empty.
    Regular(String),
    /// A delimited identifier, read as in E0D-1.4: its text without the quotes, with `""`
    /// read as `"`.
    Delimited(String),
    /// A part that starts with `"` but is not a delimited identifier (E0D-1.4), as written.
    Malformed(String),
}

impl ReferencePart {
    /// The part as an output name takes it (E1-5.1): the text of a delimited identifier
    /// without its quotes, and any other part as written.
    #[must_use]
    pub fn text(&self) -> &str {
        match self {
            Self::Regular(text) | Self::Delimited(text) | Self::Malformed(text) => text,
        }
    }
}

/// A reference as written: E1 never rejects one, and E2 reports one that names nothing as
/// `E_NAME_NOT_FOUND` (E1-3.1).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueryReference(Vec<ReferencePart>);

impl QueryReference {
    /// The reference made of `parts`, in the order written. Returns `None` when `parts` is
    /// empty: splitting any text gives at least one part.
    #[must_use]
    pub fn new(parts: Vec<ReferencePart>) -> Option<Self> {
        (!parts.is_empty()).then_some(Self(parts))
    }

    /// Every part, in the order written: at least one.
    #[must_use]
    pub fn parts(&self) -> &[ReferencePart] {
        &self.0
    }

    /// The last part.
    #[must_use]
    pub fn last_part(&self) -> Option<&ReferencePart> {
        self.0.last()
    }
}

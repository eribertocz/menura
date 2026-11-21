//! The unusable elements and the load warnings of the model (E0H-2).

use super::common::{DiagnosticCode, NormalizedName, SourceLocation};

/// The kind of an element of the model, in the order unusable elements are listed (E0H-1.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ElementKind {
    /// A dataset.
    Dataset,
    /// A field of a dataset.
    Field,
    /// A relationship.
    Relationship,
    /// A metric: of the model, or dataset-scoped.
    Metric,
}

impl ElementKind {
    /// The kind as a word.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dataset => "dataset",
            Self::Field => "field",
            Self::Relationship => "relationship",
            Self::Metric => "metric",
        }
    }
}

/// An unusable element (E0-4.1, E0-4.2a).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnusableElement {
    /// Its kind.
    pub kind: ElementKind,
    /// Its qualified name: the dataset of a field or dataset-scoped metric, then its name.
    pub name: Vec<NormalizedName>,
    /// The load warning of its own cause (E0-4 table); `None` when it is unusable only because
    /// of other elements, which reports no new warning (E0-4.2a). Never part of a hash.
    pub load_warning: Option<DiagnosticCode>,
    /// Every query error code, its own and every propagated one, ordered by code (E0-4.2a).
    pub query_errors: Vec<DiagnosticCode>,
    /// When `query_errors` holds `E0H-01`, the dataset whose row policies cannot all be
    /// propagated and the first unusable relationship that cuts them, for the message
    /// (E0-4.2a). Never part of a hash, like any diagnostic message (E0H-3.4).
    pub row_access_cut: Option<RowAccessCut>,
    /// Where it is written.
    pub location: SourceLocation,
}

/// The cause of `E0H-01`: a dataset and the first unusable relationship, by normalized name,
/// that it reaches and whose target is or reaches a dataset with `row_access` (E0-4.2a).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RowAccessCut {
    /// The dataset whose row policies cannot all be propagated.
    pub dataset: NormalizedName,
    /// The unusable relationship.
    pub relationship: NormalizedName,
}

/// One load warning (E0-4.1). Never part of a hash (E0H-3.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    /// The warning code, such as `W0D-01`.
    pub code: DiagnosticCode,
    /// The message.
    pub message: String,
    /// Where it is located.
    pub location: SourceLocation,
}

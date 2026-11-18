//! The result of the common extension rules, as data types only.

use serde_json::{Map, Value as Json};

use super::diagnostic::ExtensionDiagnostic;
use super::populations::Population;
use crate::pointer::JsonPointer;
use crate::read::Location;

/// The object types of the selected semantic model that carry `custom_extensions`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ObjectKind {
    /// The semantic model itself.
    Model,
    /// A dataset.
    Dataset,
    /// A field of a dataset.
    Field,
    /// A relationship.
    Relationship,
    /// A metric.
    Metric,
}

impl ObjectKind {
    /// The object type as the model contract names it (MDL-2), for diagnostics.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Model => "model",
            Self::Dataset => "dataset",
            Self::Field => "field",
            Self::Relationship => "relationship",
            Self::Metric => "metric",
        }
    }
}

/// The entries of ours of a model whose common extension rules hold, and the load warnings.
#[derive(Debug, Clone, PartialEq)]
pub struct Extensions {
    /// One entry per object that has an entry of ours, in document order.
    pub entries: Vec<OwnEntry>,
    /// Every population of the model, in document order (POP-2).
    pub populations: Vec<Population>,
    /// `W0C-01` warnings, in document order.
    pub warnings: Vec<ExtensionDiagnostic>,
}

/// The common extension rules do not hold: the model is rejected (E0-4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionsFailure {
    /// The rejecting diagnostics, `E0C-01` to `E0C-06`, `E0C-20` and `E0C-21`, in document order then by code; never
    /// empty.
    pub errors: Vec<ExtensionDiagnostic>,
    /// `W0C-01` warnings, in document order, reported with the errors (E0-2).
    pub warnings: Vec<ExtensionDiagnostic>,
}

/// The entry of ours of one object, ready for the validation of each extension (E0C-2.5).
#[derive(Debug, Clone, PartialEq)]
pub struct OwnEntry {
    /// The type of the object that carries the entry.
    pub object: ObjectKind,
    /// The JSON Pointer of that object, such as `/semantic_model/0/metrics/1`.
    pub object_pointer: JsonPointer,
    /// The JSON Pointer of the entry's `data` member.
    pub data_pointer: JsonPointer,
    /// Where the entry's `data` member starts in the document text.
    pub location: Location,
    /// The extension keys of `data` with their values, uninterpreted; `namespace` and `version`
    /// are removed. Every key is valid on [`Self::object`] (E0C-2.3, E0C-2.4).
    pub extensions: Map<String, Json>,
}

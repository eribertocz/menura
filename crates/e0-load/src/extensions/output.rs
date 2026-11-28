//! The result of the common extension rules, as data types only.

use serde_json::{Map, Value as Json};

use super::additivity::AdditivityDeclaration;
use super::context::{CONTEXT, ContextDeclaration};
use super::derived::DerivedDeclaration;
use super::diagnostic::ExtensionDiagnostic;
use super::populations::Population;
use super::security::Security;
use super::summaries::{AGGREGATE, SummaryDeclaration};
use super::temporal::TemporalDeclaration;
use crate::pointer::JsonPointer;
use crate::read::Location;

/// The object types of the selected semantic model that carry extension entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ObjectKind {
    /// The semantic model itself.
    Model,
    /// A dataset.
    Dataset,
    /// A field of a dataset.
    Field,
    /// A dataset-scoped metric of the development format (section 4.5).
    DatasetMetric,
    /// A relationship.
    Relationship,
    /// A metric of the semantic model.
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
            Self::DatasetMetric => "dataset metric",
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
    /// Every additivity declaration of the model, in document order (ADD-3).
    pub additivity: Vec<AdditivityDeclaration>,
    /// Every derived-metric declaration of the model, in document order (DER-3).
    pub derived: Vec<DerivedDeclaration>,
    /// Every context declaration of the model, in document order (CTX-2).
    pub context: Vec<ContextDeclaration>,
    /// The declarations of the security extension (SCY-2 to SCY-4).
    pub security: Security,
    /// Every summary declaration of the model, in document order (SUM-2).
    pub summaries: Vec<SummaryDeclaration>,
    /// Every temporal declaration of a relationship, in document order (VAL-2.1).
    pub temporal: Vec<TemporalDeclaration>,
    /// `W0C-01` warnings, in document order. `W0C-40` and `W0C-41` come from
    /// [`super::security::resolution::resolve_access`], which needs a load option.
    pub warnings: Vec<ExtensionDiagnostic>,
}

impl Extensions {
    /// Whether the metric at `metric` is marked `context`, whatever the value of the key: a
    /// context metric, which the context extension validates (DER-2.2, CTX-5.1.7).
    #[must_use]
    pub fn is_context_metric(&self, metric: &JsonPointer) -> bool {
        self.entries.iter().any(|entry| {
            entry.object == ObjectKind::Metric
                && entry.object_pointer == *metric
                && entry.extensions.contains_key(CONTEXT)
        })
    }

    /// Whether the dataset at `dataset` is a summary: its entry of ours has the key
    /// `aggregate` (SUM-6.1.1).
    #[must_use]
    pub fn is_summary(&self, dataset: &JsonPointer) -> bool {
        self.entries.iter().any(|entry| {
            entry.object == ObjectKind::Dataset
                && entry.object_pointer == *dataset
                && entry.extensions.contains_key(AGGREGATE)
        })
    }

    /// The derived-metric declaration of the metric at `metric`, when it has one.
    #[must_use]
    pub fn derived_declaration(&self, metric: &JsonPointer) -> Option<&DerivedDeclaration> {
        self.derived
            .iter()
            .find(|declaration| declaration.metric == *metric)
    }

    /// The temporal declaration of the relationship at `relationship`, when it is a temporal
    /// relationship (VAL-2.1).
    #[must_use]
    pub fn temporal_declaration(&self, relationship: &JsonPointer) -> Option<&TemporalDeclaration> {
        self.temporal
            .iter()
            .find(|declaration| declaration.relationship == *relationship)
    }

    /// The context declaration of the metric at `metric`, when it has one.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "E0.h finds context declarations through its own index; only the extension tests call it"
        )
    )]
    #[must_use]
    pub fn context_declaration(&self, metric: &JsonPointer) -> Option<&ContextDeclaration> {
        self.context
            .iter()
            .find(|declaration| declaration.metric == *metric)
    }
}

/// The common extension rules do not hold: the model is rejected (E0-4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionsFailure {
    /// The rejecting diagnostics, `E0C-01` to `E0C-06`, `E0C-10`, `E0C-11`, `E0C-13` to
    /// `E0C-15`, `E0C-20`, `E0C-21`, `E0C-36`, `E0C-41` to `E0C-45`, `E0C-47`, `E0C-50`,
    /// `E0C-55`, `E0C-56` and `E0C-62`, in document order then by code; never empty.
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

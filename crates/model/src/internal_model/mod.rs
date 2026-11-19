//! The immutable internal model built by E0.h (E0H-1, E0H-2): everything E1–E6 and `describe`
//! need, so no later stage reads the model document (E0H-1.2).
//!
//! These types hold data only (S9); E0.h assembles them and computes the hashes. Every
//! collection of named elements is ordered by normalized name (E0H-1.4), every element keeps
//! its source location (E0H-1.5), and expressions are normalized trees (E0H-1.3).

mod check;
mod common;
mod dataset;
mod diagnostics;
mod metric;
mod relationship;
mod security;
mod summary;

pub use check::{CheckKind, RecordedCheck};
pub use common::{
    Descriptions, DiagnosticCode, FieldPath, NormalizedName, SourceLocation, TypedValue, ValueType,
};
pub use dataset::{Dataset, Field, PartialPeriod, Population, PopulationMode};
pub use diagnostics::{ElementKind, UnusableElement, Warning};
pub use metric::{
    Additivity, Component, ContextMetric, ContextModifier, DeclaredAdditivity,
    DecomposabilityClass, DerivedMetric, DivisionByZero, Metric, MetricClassification,
    MissingHandling, NullHandling, RemovedDimensions, Selection, SemiAdditive, SnapshotAggregation,
    StaleHandling, UnexpectedHandling,
};
pub use relationship::{
    DatasetPair, DatasetReach, GraphEdge, PathCount, ReachedDataset, Relationship,
    RelationshipGraph,
};
pub use security::{
    AccessMode, AttributeType, DatasetAccess, FieldRestriction, RoleName, RowPolicy, ScalarType,
    SecurityAttribute, SecurityContext,
};
pub use summary::{Summary, SummaryDimension, SummaryMeasure};

use semantic_compiler_contracts::Sha256Hash;

/// The parts of an internal model, before it is sealed with its hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelParts {
    /// The name of the semantic model.
    pub name: NormalizedName,
    /// The `description` and `ai_context` of the semantic model.
    pub descriptions: Descriptions,
    /// The datasets, ordered by name.
    pub datasets: Vec<Dataset>,
    /// The relationships, usable or not, ordered by name.
    pub relationships: Vec<Relationship>,
    /// The graph of the usable relationships (E0H-2.3).
    pub graph: RelationshipGraph,
    /// The metrics of the semantic model, ordered by name.
    pub metrics: Vec<Metric>,
    /// The `security_context`, when the model has one.
    pub security_context: Option<SecurityContext>,
    /// The summaries, ordered by summary dataset name.
    pub summaries: Vec<Summary>,
    /// The recorded integrity checks, ordered by `check_id` (E0H-4.3).
    pub checks: Vec<RecordedCheck>,
    /// The unusable elements, ordered by kind then by qualified name (E0H-1.4).
    pub unusable: Vec<UnusableElement>,
    /// Every load warning of E0, in report order (E0-4.3).
    pub warnings: Vec<Warning>,
}

/// The internal model: immutable once built (E0H-1.1). A new model is a new value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalModel {
    parts: ModelParts,
    hash: Sha256Hash,
}

impl InternalModel {
    /// Seals `parts` with `hash`, the model hash computed over them (E0H-3.2).
    #[must_use]
    pub fn new(parts: ModelParts, hash: Sha256Hash) -> Self {
        Self { parts, hash }
    }

    /// The model hash (E0H-3.2, MDL-3.1).
    #[must_use]
    pub fn hash(&self) -> &Sha256Hash {
        &self.hash
    }

    /// Every part of the model, read-only.
    #[must_use]
    pub fn parts(&self) -> &ModelParts {
        &self.parts
    }

    /// The dataset named `name`.
    #[must_use]
    pub fn dataset(&self, name: &NormalizedName) -> Option<&Dataset> {
        find(&self.parts.datasets, name, |dataset| &dataset.name)
    }

    /// The metric of the semantic model named `name`.
    #[must_use]
    pub fn metric(&self, name: &NormalizedName) -> Option<&Metric> {
        find(&self.parts.metrics, name, |metric| &metric.name)
    }

    /// The relationship named `name`.
    #[must_use]
    pub fn relationship(&self, name: &NormalizedName) -> Option<&Relationship> {
        find(&self.parts.relationships, name, |relationship| {
            &relationship.name
        })
    }
}

impl Dataset {
    /// The field named `name`.
    #[must_use]
    pub fn field(&self, name: &NormalizedName) -> Option<&Field> {
        find(&self.fields, name, |field| &field.name)
    }
}

/// The element named `name` of `elements`, which are ordered by name.
fn find<'model, Element>(
    elements: &'model [Element],
    name: &NormalizedName,
    name_of: impl Fn(&Element) -> &NormalizedName,
) -> Option<&'model Element> {
    elements
        .binary_search_by(|element| name_of(element).cmp(name))
        .ok()
        .and_then(|index| elements.get(index))
}

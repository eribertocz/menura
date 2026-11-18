//! The derived metrics of a model and their inputs, as E0.d resolves them, as data types only.

use semantic_compiler_model::expression::Reference;

use crate::extensions::ExtensionDiagnostic;
use crate::extensions::derived::DivisionByZero;
use crate::pointer::JsonPointer;
use crate::read::Location;

/// The derived metrics of a model whose inputs resolve, and the metrics `E0C-37` makes
/// unusable.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DerivedMetrics {
    /// Every derived metric whose formula parses, in document order.
    pub metrics: Vec<DerivedMetric>,
    /// The metrics with `AGG(…)` that are neither derived nor context metrics, in document
    /// order: each is unusable, and a query that names it fails with `E0C-37` (E0-4 table).
    pub unusable: Vec<JsonPointer>,
    /// One `E0C-37` diagnostic per such metric, located at the metric, in document order. Each
    /// is reported as `W0G-01`.
    pub warnings: Vec<ExtensionDiagnostic>,
}

impl DerivedMetrics {
    /// The derived metric at `pointer`, when it is one.
    #[must_use]
    pub fn metric(&self, pointer: &JsonPointer) -> Option<&DerivedMetric> {
        self.metrics
            .iter()
            .find(|metric| metric.pointer == *pointer)
    }
}

/// The inputs of a derived metric do not resolve: the model is rejected (E0-4 table).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedRejection {
    /// The `E0C-32` and `E0C-33` diagnostics, in document order then by code; never empty.
    pub errors: Vec<ExtensionDiagnostic>,
    /// The `E0C-37` diagnostics found with them, as in [`DerivedMetrics::warnings`].
    pub warnings: Vec<ExtensionDiagnostic>,
}

/// One derived metric (DER-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedMetric {
    /// The JSON Pointer of the metric.
    pub pointer: JsonPointer,
    /// Its name as written, for messages.
    pub name: String,
    /// One input per `AGG(…)` term of its formula, in the order they are written; never empty.
    pub inputs: Vec<DerivedInput>,
    /// Why the metric is derived, with its division rule.
    pub origin: DerivedOrigin,
    /// Where the diagnostics about the metric's formula are located: the `data` member of its
    /// entry of ours for a declared metric (E0C-2.6), its `expression` otherwise.
    pub diagnostic_pointer: JsonPointer,
    /// Where that node starts in the document text.
    pub diagnostic_location: Location,
}

/// Why a metric is derived (DER-2.2, DER-5.1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DerivedOrigin {
    /// It is marked by the `derived` extension, with its `on_division_by_zero` when given.
    Declared(Option<DivisionByZero>),
    /// It is a development-line model metric whose `AGG(…)` terms all name dataset metrics
    /// (`AGG(dataset.metric)`): it has no `on_division_by_zero`, and its division follows
    /// E0F-6.1.
    Development,
}

/// One `AGG(…)` term of a derived formula and the metric it names (DER-5.1.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedInput {
    /// The reference inside `AGG(…)`, as written.
    pub reference: Reference,
    /// The JSON Pointer of the metric it names: a model metric, or a dataset metric for
    /// `AGG(dataset.metric)`.
    pub metric: JsonPointer,
}

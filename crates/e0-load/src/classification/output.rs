//! The result of metric classification (E0G-6), as data types only.

use semantic_compiler_contracts::types::common::LogicalType;
use semantic_compiler_model::expression::{AggregateCall, Expression};

use super::diagnostic::{ClassificationCode, ClassificationDiagnostic};
use crate::names::Name;
use crate::pointer::JsonPointer;

/// The classification of a model's base metrics: each usable metric split into its components,
/// the metrics an E0G-2 rule makes unusable, and their warnings. Classification never rejects
/// the model: every E0G-2 problem makes its metric unusable (E0-4 table).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Classification {
    /// Every metric usable after E0.f that breaks no E0G-2 rule, in document order (E0G-6).
    pub metrics: Vec<ClassifiedMetric>,
    /// One entry per metric an E0G-2 rule makes unusable, in document order.
    pub unusable: Vec<UnusableMetric>,
    /// One diagnostic per unusable metric, located at the metric, in document order (E0G-2.8).
    /// Each is reported as [`ClassificationCode::WARNING`].
    pub warnings: Vec<ClassificationDiagnostic>,
}

/// What kind of metric a classification is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MetricKind {
    /// A metric of the semantic model.
    Metric,
    /// A dataset-scoped metric of the development format (section 4.5).
    DatasetMetric,
}

/// One classified metric (E0G-6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassifiedMetric {
    /// The kind of metric.
    pub kind: MetricKind,
    /// Its JSON Pointer, such as `/semantic_model/0/metrics/2`.
    pub pointer: JsonPointer,
    /// Its components, one per aggregate, in written order (E0G-1.1); never empty.
    pub components: Vec<Component>,
    /// Its deduced additivity (E0G-4).
    pub additivity: Additivity,
    /// Whether it can be recomposed from partial results: only when every component can
    /// (E0G-3.1).
    pub recomposable: bool,
    /// Its result type, as E0.f found it.
    pub result_type: LogicalType,
    /// Whether it can be NULL, as E0.f found it.
    pub nullable: bool,
}

/// One aggregate of a metric expression (E0G-1.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// The aggregate call.
    pub call: AggregateCall,
    /// Its `FILTER (WHERE …)` condition, if written.
    pub filter: Option<Expression>,
    /// The dataset whose rows the aggregate is evaluated over (E0G-1.1, E0G-1.4).
    pub home_dataset: Name,
    /// Its decomposability class (E0G-3); a `FILTER` does not change it (E0G-3.2).
    pub class: DecomposabilityClass,
}

/// The decomposability class of an aggregate (E0G-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecomposabilityClass {
    /// `SUM`, `COUNT`, `MIN`, `MAX`: recomposable with the same function.
    Distributive,
    /// `AVG`, `STDDEV*`, `VAR*`: computed from distributive parts.
    Algebraic,
    /// `MEDIAN`, `PERCENTILE_*`, `COUNT(DISTINCT)`, `SUM(DISTINCT)`: always from base rows.
    Holistic,
}

/// Over which dimensions a metric may be re-aggregated (E0G-4). Only the additivity extension
/// makes a metric semi-additive; without it, additivity is deduced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Additivity {
    /// Every component is distributive.
    Additive,
    /// A component is algebraic or holistic: the metric is always computed from base rows.
    NonAdditive,
}

/// A metric marked unusable by an E0G-2 rule (E0-4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnusableMetric {
    /// The kind of metric.
    pub kind: MetricKind,
    /// Its JSON Pointer.
    pub pointer: JsonPointer,
    /// The query error of a query that names it: the first of the rules it breaks in the
    /// precedence of E2-5 (E0G-2.8).
    pub query_error: ClassificationCode,
}

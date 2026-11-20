//! The result of metric classification (E0G-6), as data types only.

use menura_contracts::types::common::LogicalType;
use menura_model::expression::{AggregateCall, Expression};

use std::collections::BTreeSet;

use super::diagnostic::{ClassificationCode, ClassificationDiagnostic};
use crate::expressions::ExpressionDiagnosticCode;
use crate::extensions::ExtensionCode;
use crate::extensions::additivity::DeclaredAdditivity;
use crate::names::Name;
use crate::pointer::JsonPointer;
use crate::references::ReferenceCode;
use menura_expressions::typing::TypingCode;

/// The classification of a model's metrics: each usable base metric split into its components,
/// each usable derived metric with its inputs, the metrics an E0G-2 rule, an additivity
/// conflict or an unusable input makes unusable, and their warnings. Classification never
/// rejects the model: every problem makes its metric unusable (E0-4 table).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Classification {
    /// Every base metric usable after E0.f that breaks no E0G-2 rule and whose additivity
    /// declaration holds, and every derived metric typed in E0.f whose inputs are all usable,
    /// in document order (E0G-6).
    pub metrics: Vec<ClassifiedMetric>,
    /// One entry per metric an E0G-2 rule, `E0G-01` or an unusable input (DER-5.1.5) makes
    /// unusable, in document order.
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
    /// Its components, one per aggregate, in written order (E0G-1.1); never empty for a base
    /// metric, and empty for a derived metric, which has none of its own.
    pub components: Vec<Component>,
    /// The metrics a derived metric reads with `AGG(…)`, each once, in written order
    /// (DER-2.1); empty for a base metric.
    pub inputs: Vec<JsonPointer>,
    /// Its home datasets: those of its components, or for a derived metric the union of those
    /// of its inputs (E0G-5.2).
    pub home_datasets: BTreeSet<Name>,
    /// Its additivity: the declared one when the additivity extension declares it, otherwise
    /// the deduced one (E0G-4, E0G-4.3); always non-additive for a derived metric, whose value
    /// is never re-aggregated (DER-5.1.6).
    pub additivity: Additivity,
    /// The additivity declaration of the metric that E0.c validated, when it has one
    /// (E0G-4.3); a declaration that conflicts with the deduced additivity makes the metric
    /// unusable instead (`E0G-01`).
    pub declaration: Option<DeclaredAdditivity>,
    /// Whether it can be recomposed from partial results: only when every component can
    /// (E0G-3.1), or for a derived metric when every input can (E0G-5.3).
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
    /// A component is algebraic or holistic, or the metric is declared `non_additive`: it is
    /// always computed from base rows.
    NonAdditive,
    /// The metric is declared `semi_additive`: it is not summed across `over` (ADD-3.2).
    SemiAdditive,
}

/// A metric marked unusable by an E0G-2 rule, an additivity conflict or an unusable input
/// (E0-4.1, DER-5.1.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnusableMetric {
    /// The kind of metric.
    pub kind: MetricKind,
    /// Its JSON Pointer.
    pub pointer: JsonPointer,
    /// The query error of a query that names it: the first of the rules it breaks in the
    /// precedence of E2-5 (E0G-2.8), or the query error of its first unusable input.
    pub query_error: QueryError,
}

/// The query error of an unusable metric: the code of the substage that made it unusable.
///
/// A derived metric with an unusable input takes the query error of its first unusable input
/// in written order (DER-5.1.5), whichever substage made that input unusable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryError {
    /// `E0B-08` or `E0B-09`, from the structure of its expression (E0-4 table).
    Structure(&'static str),
    /// `E0F-01`, `E0F-02` or `E_DEFERRED_WINDOWED_METRIC`, from parsing its expression.
    Expression(ExpressionDiagnosticCode),
    /// `E0D-03` to `E0D-07` or `E0F-02`, from its references.
    Reference(ReferenceCode),
    /// `E0C-37`: `AGG(…)` outside a derived metric.
    Extension(ExtensionCode),
    /// An `E0F` code from typing.
    Typing(TypingCode),
    /// A rule of E0.g.
    Classification(ClassificationCode),
}

impl QueryError {
    /// The code as the specification writes it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Structure(code) => code,
            Self::Expression(code) => code.as_str(),
            Self::Reference(code) => code.as_str(),
            Self::Extension(code) => code.as_str(),
            Self::Typing(code) => code.as_str(),
            Self::Classification(code) => code.as_str(),
        }
    }
}

impl From<ClassificationCode> for QueryError {
    fn from(code: ClassificationCode) -> Self {
        Self::Classification(code)
    }
}

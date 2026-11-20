//! Metrics: normalized trees, components, classes, additivity, types and the interpreted
//! metric extensions (E0H-2 "Metrics").

use std::num::NonZeroU64;

use menura_contracts::types::common::Grain;

use super::common::{
    Descriptions, DiagnosticCode, FieldPath, NormalizedName, SourceLocation, TypedValue,
};
use crate::expression::{AggregateCall, Expression};

/// One metric: of the semantic model, or dataset-scoped in the development format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metric {
    /// Its name.
    pub name: NormalizedName,
    /// Its normalized expression tree (E0H-1.3), when its compiled variant parses.
    pub expression: Option<Expression>,
    /// Its result type and nullability, when E0.f typed it.
    pub typed: Option<TypedValue>,
    /// Its classification (E0G-6), when E0.g classified it.
    pub classification: Option<MetricClassification>,
    /// Its additivity declaration (ADD-3), when E0.c validated one.
    pub declared_additivity: Option<DeclaredAdditivity>,
    /// Its derived-metric extension (DER-3), when it is a derived metric.
    pub derived: Option<DerivedMetric>,
    /// Its context extension (CTX-2), when it is a context metric.
    pub context: Option<ContextMetric>,
    /// Its `description`, `label` and `ai_context`.
    pub descriptions: Descriptions,
    /// The query error codes of a query that names it, ordered by code; empty when it is
    /// usable (E0-4.1, E0-4.2a).
    pub query_errors: Vec<DiagnosticCode>,
    /// Where it is written.
    pub location: SourceLocation,
}

/// What E0.g found for a metric (E0G-6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricClassification {
    /// Its components, one per aggregate, in written order; empty for a derived metric.
    pub components: Vec<Component>,
    /// The metrics a derived metric reads with `AGG(…)`, in written order.
    pub inputs: Vec<NormalizedName>,
    /// Its home datasets, ordered by name (E0G-5.2).
    pub home_datasets: Vec<NormalizedName>,
    /// Its additivity (E0G-4).
    pub additivity: Additivity,
    /// Whether it can be recomposed from partial results (E0G-3.1, E0G-5.3).
    pub recomposable: bool,
}

/// One aggregate of a metric expression (E0G-1.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// The aggregate call.
    pub call: AggregateCall,
    /// Its `FILTER (WHERE …)` condition, when written.
    pub filter: Option<Expression>,
    /// The dataset whose rows it is evaluated over.
    pub home_dataset: NormalizedName,
    /// Its decomposability class (E0G-3).
    pub class: DecomposabilityClass,
}

/// The decomposability class of an aggregate (E0G-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecomposabilityClass {
    /// `SUM`, `COUNT`, `MIN`, `MAX`.
    Distributive,
    /// `AVG`, `STDDEV*`, `VAR*`.
    Algebraic,
    /// `MEDIAN`, `PERCENTILE_*`, `COUNT(DISTINCT)`, `SUM(DISTINCT)`.
    Holistic,
}

/// Over which dimensions a metric may be re-aggregated (E0G-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Additivity {
    /// Every component is distributive.
    Additive,
    /// Always computed from base rows.
    NonAdditive,
    /// Not summed across `over` (ADD-3.2).
    SemiAdditive,
}

/// The value of `additivity.type`, with the keys of `semi_additive` (ADD-3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclaredAdditivity {
    /// `non_additive`.
    NonAdditive,
    /// `semi_additive`.
    SemiAdditive(SemiAdditive),
}

/// The keys of a `semi_additive` declaration (ADD-3.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemiAdditive {
    /// The field across which the metric is not summed.
    pub over: FieldPath,
    /// The fields that identify each snapshot entity, in the order written.
    pub entity: Vec<FieldPath>,
    /// How values are combined across `over`.
    pub aggregation: SnapshotAggregation,
    /// What a NULL value means.
    pub nulls: NullHandling,
    /// The grain of the snapshots, when given.
    pub snapshot_grain: Option<Grain>,
    /// The expected population, by normalized name, when given.
    pub population: Option<NormalizedName>,
    /// What an expected entity without a row contributes, when given.
    pub missing: Option<MissingHandling>,
    /// What a row of an unexpected entity does, when given.
    pub unexpected: Option<UnexpectedHandling>,
}

/// The values of `aggregation` (ADD-3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SnapshotAggregation {
    /// `last`, with its `selection`.
    Last(Selection),
    /// `first`, with its `selection`.
    First(Selection),
    /// `min`.
    Minimum,
    /// `max`.
    Maximum,
    /// `avg`.
    Average,
}

/// The values of `selection` (ADD-3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Selection {
    /// `global_period`.
    GlobalPeriod,
    /// `per_entity`.
    PerEntity,
}

/// The values of `nulls` (ADD-3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NullHandling {
    /// `as_missing`.
    AsMissing,
    /// `keep`.
    Keep,
    /// `zero`.
    Zero,
    /// `error`.
    Error,
}

/// The values of `missing` (ADD-3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MissingHandling {
    /// `exclude`.
    Exclude,
    /// `carry_forward`, with its keys.
    CarryForward {
        /// The oldest value carried, in `snapshot_grain` periods.
        max_carry_forward: NonZeroU64,
        /// What an older value does.
        on_stale: StaleHandling,
    },
    /// `zero`.
    Zero,
    /// `error`.
    Error,
}

/// The values of `on_stale` (ADD-3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StaleHandling {
    /// `exclude`.
    Exclude,
    /// `error`.
    Error,
}

/// The values of `unexpected` (ADD-3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnexpectedHandling {
    /// `include`.
    Include,
    /// `exclude`.
    Exclude,
    /// `error`.
    Error,
}

/// The derived-metric extension of a metric (DER-3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedMetric {
    /// The metrics its formula reads with `AGG(…)`, each once, in written order.
    pub inputs: Vec<NormalizedName>,
    /// The value of `on_division_by_zero`, when given.
    pub on_division_by_zero: Option<DivisionByZero>,
}

/// The values of `on_division_by_zero` (DER-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DivisionByZero {
    /// `null`.
    Null,
    /// `error`.
    Error,
}

/// The context extension of a metric (CTX-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextMetric {
    /// The metric evaluated in the modified context.
    pub base: NormalizedName,
    /// The modifier.
    pub modifier: ContextModifier,
}

/// The modifier of a context metric (CTX-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextModifier {
    /// `period_to_date` (CTX-3a.1).
    PeriodToDate {
        /// The time field.
        time_dimension: FieldPath,
        /// The grain of the period.
        grain: Grain,
    },
    /// `prior_period` (CTX-3a.2).
    PriorPeriod {
        /// The time field.
        time_dimension: FieldPath,
        /// The grain of the period.
        grain: Grain,
        /// The number of `grain` units to shift back.
        offset: NonZeroU64,
    },
    /// `share_of_total` (CTX-3a.3).
    ShareOfTotal(RemovedDimensions),
}

/// The value of `remove` of a `share_of_total` modifier (CTX-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemovedDimensions {
    /// `all`.
    All,
    /// The fields taken out of the grouping, in the order written.
    Fields(Vec<FieldPath>),
}

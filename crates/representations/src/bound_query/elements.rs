//! The elements of a bound query, each resolved to model elements by normalized name.

use menura_contracts::types::common::Grain;
use menura_contracts::types::query::{Direction, NullsOrder};
use menura_model::expression::{DateValue, Expression};
use menura_model::internal_model::{FieldPath, NormalizedName};

use crate::names::OutputName;
use crate::origin::QueryLocation;

/// A metric of the model: of the semantic model, or dataset-scoped (development format).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MetricPath {
    /// A metric of the semantic model.
    SemanticModel(NormalizedName),
    /// A dataset-scoped metric.
    Dataset {
        /// The dataset that declares it.
        dataset: NormalizedName,
        /// The metric.
        metric: NormalizedName,
    },
}

/// A dimension resolved against the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundDimension {
    /// What the dimension reads.
    pub kind: BoundDimensionKind,
    /// Its output name (E1-5.1).
    pub output_name: OutputName,
    /// Where it is written.
    pub origin: QueryLocation,
}

/// What a bound dimension reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundDimensionKind {
    /// A field, with an optional time grain.
    Field {
        /// The field.
        field: FieldPath,
        /// The time grain applied to it, when the query has one.
        time_grain: Option<Grain>,
        /// Where the `time_grains` entry of `time_grain` is written, when the query has one
        /// (E4-7.8).
        time_grain_origin: Option<QueryLocation>,
    },
    /// SQL form: a scalar expression over fields of one dataset (E2-3.3).
    Expression {
        /// The dataset whose fields it reads.
        dataset: NormalizedName,
        /// The expression.
        expression: Expression,
    },
}

/// A metric resolved against the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundMetric {
    /// The metric.
    pub kind: BoundMetricKind,
    /// Its output name (E1-5.1).
    pub output_name: OutputName,
    /// Where it is written.
    pub origin: QueryLocation,
}

/// How a bound metric is defined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundMetricKind {
    /// A metric of the model.
    Reference(MetricPath),
    /// An inline expression over metrics.
    Inline(Expression),
}

/// A filter assigned to the datasets it reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundFilter {
    /// The expression.
    pub expression: Expression,
    /// The datasets it reads, ordered by name; empty when it reads none (E3-3.6).
    pub datasets: Vec<NormalizedName>,
    /// Where it is written.
    pub origin: QueryLocation,
}

/// A date range with its bounds resolved to dates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundDateRange {
    /// The field it applies to.
    pub field: FieldPath,
    /// The first date, inclusive.
    pub from: DateValue,
    /// The last date, inclusive.
    pub to: DateValue,
    /// Where it is written.
    pub origin: QueryLocation,
}

/// A relationship path chosen by the query, resolved to model elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundPath {
    from: NormalizedName,
    to: NormalizedName,
    relationships: Vec<NormalizedName>,
    origin: QueryLocation,
}

impl BoundPath {
    /// The path from `from` to `to` through `relationships`, in order. Returns `None` when
    /// `relationships` is empty.
    #[must_use]
    pub fn new(
        from: NormalizedName,
        to: NormalizedName,
        relationships: Vec<NormalizedName>,
        origin: QueryLocation,
    ) -> Option<Self> {
        (!relationships.is_empty()).then_some(Self {
            from,
            to,
            relationships,
            origin,
        })
    }

    /// The starting dataset.
    #[must_use]
    pub fn from(&self) -> &NormalizedName {
        &self.from
    }

    /// The ending dataset.
    #[must_use]
    pub fn to(&self) -> &NormalizedName {
        &self.to
    }

    /// The relationships, in order: at least one.
    #[must_use]
    pub fn relationships(&self) -> &[NormalizedName] {
        &self.relationships
    }

    /// Where it is written.
    #[must_use]
    pub fn origin(&self) -> &QueryLocation {
        &self.origin
    }
}

/// An ordering term resolved to an output column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundOrderItem {
    /// The position of the output column it orders by, from 0.
    pub column: usize,
    /// The direction.
    pub direction: Direction,
    /// Where NULLs go.
    pub nulls: NullsOrder,
    /// Where it is written.
    pub origin: QueryLocation,
}

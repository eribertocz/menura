//! The elements of a query tree (E1-5), each as written with its origin.

use menura_contracts::types::common::Grain;
use menura_contracts::types::query::{DateBounds, Direction, NullsOrder};
use menura_model::expression::Expression;

use super::reference::QueryReference;
use crate::names::OutputName;
use crate::origin::QueryLocation;

/// A requested dimension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryDimension {
    /// What the dimension reads.
    pub kind: QueryDimensionKind,
    /// Its output name (E1-5.1).
    pub output_name: OutputName,
    /// Where it is written.
    pub origin: QueryLocation,
}

/// What a dimension reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryDimensionKind {
    /// A field reference. Its time grain, when written, is in the tree's time grains: E2 matches
    /// grains to dimensions (QRY-4.1).
    Field {
        /// The field, as written.
        reference: QueryReference,
    },
    /// SQL form: a scalar expression over fields of one dataset.
    Expression(Expression),
}

/// A `time_grains` entry, as written (QRY-4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeGrainChoice {
    /// The dimension it applies to, as written.
    pub dimension: QueryReference,
    /// The grain.
    pub grain: Grain,
    /// Where it is written.
    pub origin: QueryLocation,
}

/// A requested metric.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryMetric {
    /// The metric.
    pub kind: QueryMetricKind,
    /// Its output name (E1-5.1).
    pub output_name: OutputName,
    /// Where it is written.
    pub origin: QueryLocation,
}

/// How a metric is requested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryMetricKind {
    /// A reference to a metric of the model, as written.
    Reference(QueryReference),
    /// An inline expression over metrics.
    Inline(Expression),
}

/// A filter: a parsed expression, assigned to its datasets in E2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryFilter {
    /// The parsed expression.
    pub expression: Expression,
    /// Where it is written.
    pub origin: QueryLocation,
}

/// A date range on a dimension, as written (QRY-4.2); E2 resolves a relative one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryDateRange {
    /// The dimension, as written.
    pub dimension: QueryReference,
    /// The absolute bounds or the relative range, as written.
    pub bounds: DateBounds,
    /// Where it is written.
    pub origin: QueryLocation,
}

/// A relationship path chosen by the query (QRY-4.3), as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryPath {
    from: QueryReference,
    to: QueryReference,
    relationships: Vec<QueryReference>,
    origin: QueryLocation,
}

impl QueryPath {
    /// The path from `from` to `to` through `relationships`, in order. Returns `None` when
    /// `relationships` is empty: a path names at least one relationship (QRY-4.3).
    #[must_use]
    pub fn new(
        from: QueryReference,
        to: QueryReference,
        relationships: Vec<QueryReference>,
        origin: QueryLocation,
    ) -> Option<Self> {
        (!relationships.is_empty()).then_some(Self {
            from,
            to,
            relationships,
            origin,
        })
    }

    /// The starting dataset, as written.
    #[must_use]
    pub fn from(&self) -> &QueryReference {
        &self.from
    }

    /// The ending dataset, as written.
    #[must_use]
    pub fn to(&self) -> &QueryReference {
        &self.to
    }

    /// The relationships, in order: at least one.
    #[must_use]
    pub fn relationships(&self) -> &[QueryReference] {
        &self.relationships
    }

    /// Where it is written.
    #[must_use]
    pub fn origin(&self) -> &QueryLocation {
        &self.origin
    }
}

/// One ordering term (QRY-4.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryOrderItem {
    /// The output column it orders by, as written.
    pub output: OutputName,
    /// The direction.
    pub direction: Direction,
    /// Where NULLs go.
    pub nulls: NullsOrder,
    /// Where it is written.
    pub origin: QueryLocation,
}

/// The row limit (QRY-4.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryLimit {
    /// The maximum number of rows.
    pub rows: u64,
    /// Where it is written.
    pub origin: QueryLocation,
}

//! The query tree: the output of E1 (E1-5), with every element as written and its origin.

mod elements;
#[cfg(test)]
mod tests;

pub use elements::{
    QueryDateRange, QueryDimension, QueryDimensionKind, QueryFilter, QueryLimit, QueryMetric,
    QueryMetricKind, QueryOrderItem, QueryPath, TimeGrainChoice,
};

use crate::columns::{Columns, OutputColumn};

/// The elements of a query tree other than its output columns (E1-5).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QueryTreeParts {
    /// The filters, in the order written: `filter`, then the `filters` extension.
    pub filters: Vec<QueryFilter>,
    /// The date range, when written.
    pub date_range: Option<QueryDateRange>,
    /// The relationships chosen by the query, in the order written.
    pub paths: Vec<QueryPath>,
    /// The ordering terms, in the order written; empty when the query has none.
    pub order: Vec<QueryOrderItem>,
    /// The row limit, when written.
    pub limit: Option<QueryLimit>,
}

/// A parsed query (E1-5): its output columns in output order, and its other elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryTree {
    columns: Columns<QueryDimension, QueryMetric>,
    parts: QueryTreeParts,
}

impl QueryTree {
    /// The tree of a wide-table query: its columns are the dimensions, then the metrics
    /// (E1-5 "Column order").
    #[must_use]
    pub fn wide_table(
        dimensions: Vec<QueryDimension>,
        metrics: Vec<QueryMetric>,
        parts: QueryTreeParts,
    ) -> Self {
        Self {
            columns: Columns::dimensions_then_metrics(dimensions, metrics),
            parts,
        }
    }

    /// The tree of a SQL-form query: its columns are in `SELECT` order (E1-5 "Column order").
    #[must_use]
    pub fn in_select_order(
        columns: Vec<OutputColumn<QueryDimension, QueryMetric>>,
        parts: QueryTreeParts,
    ) -> Self {
        Self {
            columns: Columns::in_select_order(columns),
            parts,
        }
    }

    /// The output columns, in output order.
    #[must_use]
    pub fn columns(&self) -> &Columns<QueryDimension, QueryMetric> {
        &self.columns
    }

    /// The filters, in the order written.
    #[must_use]
    pub fn filters(&self) -> &[QueryFilter] {
        &self.parts.filters
    }

    /// The date range, when written.
    #[must_use]
    pub fn date_range(&self) -> Option<&QueryDateRange> {
        self.parts.date_range.as_ref()
    }

    /// The relationships chosen by the query, in the order written.
    #[must_use]
    pub fn paths(&self) -> &[QueryPath] {
        &self.parts.paths
    }

    /// The ordering terms, in the order written.
    #[must_use]
    pub fn order(&self) -> &[QueryOrderItem] {
        &self.parts.order
    }

    /// The row limit, when written.
    #[must_use]
    pub fn limit(&self) -> Option<&QueryLimit> {
        self.parts.limit.as_ref()
    }
}

//! The bound query: the output of E2 (E2-2). Every reference points to a model element, and
//! every authorization decision and the authorized subgraph are recorded.

mod authorization;
mod elements;
#[cfg(test)]
mod tests;

pub use authorization::{
    AuthorizationDecision, AuthorizationRule, AuthorizedObject, AuthorizedSubgraph,
};
pub use elements::{
    BoundDateRange, BoundDimension, BoundDimensionKind, BoundFilter, BoundMetric, BoundMetricKind,
    BoundOrderItem, BoundPath, MetricPath,
};

use crate::columns::Columns;
use crate::query_tree::QueryLimit;
use crate::time::QueryTime;

/// The elements of a bound query other than its output columns.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BoundQueryParts {
    /// The filters, in the order of the query tree, each assigned to its datasets.
    pub filters: Vec<BoundFilter>,
    /// The date range, with its bounds resolved, when the query has one.
    pub date_range: Option<BoundDateRange>,
    /// The relationships chosen by the query, in the order of the query tree.
    pub paths: Vec<BoundPath>,
    /// The ordering terms, in the order of the query tree.
    pub order: Vec<BoundOrderItem>,
    /// The row limit, when the query has one.
    pub limit: Option<QueryLimit>,
    /// Every authorization decision, in the order taken (E2-4.7).
    pub authorization: Vec<AuthorizationDecision>,
    /// The authorized subgraph (E2-4.8).
    pub subgraph: AuthorizedSubgraph,
}

/// A query whose every name is resolved and authorized (E2-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundQuery {
    columns: Columns<BoundDimension, BoundMetric>,
    time: QueryTime,
    parts: BoundQueryParts,
}

impl BoundQuery {
    /// The bound query with output `columns`, in the order of the query tree, the time zone
    /// and reference time of the query tree, and `parts`.
    #[must_use]
    pub fn new(
        columns: Columns<BoundDimension, BoundMetric>,
        time: QueryTime,
        parts: BoundQueryParts,
    ) -> Self {
        Self {
            columns,
            time,
            parts,
        }
    }

    /// The time zone and the reference time, which resolve relative ranges (QRY-4.2) and
    /// truncate time grains (E6-5.2).
    #[must_use]
    pub fn time(&self) -> &QueryTime {
        &self.time
    }

    /// The output columns, in output order.
    #[must_use]
    pub fn columns(&self) -> &Columns<BoundDimension, BoundMetric> {
        &self.columns
    }

    /// The filters, each assigned to its datasets.
    #[must_use]
    pub fn filters(&self) -> &[BoundFilter] {
        &self.parts.filters
    }

    /// The date range, when the query has one.
    #[must_use]
    pub fn date_range(&self) -> Option<&BoundDateRange> {
        self.parts.date_range.as_ref()
    }

    /// The relationships chosen by the query.
    #[must_use]
    pub fn paths(&self) -> &[BoundPath] {
        &self.parts.paths
    }

    /// The ordering terms.
    #[must_use]
    pub fn order(&self) -> &[BoundOrderItem] {
        &self.parts.order
    }

    /// The row limit, when the query has one.
    #[must_use]
    pub fn limit(&self) -> Option<&QueryLimit> {
        self.parts.limit.as_ref()
    }

    /// Every authorization decision, in the order taken (E2-4.7).
    #[must_use]
    pub fn authorization(&self) -> &[AuthorizationDecision] {
        &self.parts.authorization
    }

    /// The authorized subgraph (E2-4.8).
    #[must_use]
    pub fn subgraph(&self) -> &AuthorizedSubgraph {
        &self.parts.subgraph
    }
}

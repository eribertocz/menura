//! What rows 3 to 9 of E2-5 produce: every name resolved and authorized.

use menura_contracts::types::common::Grain;
use menura_contracts::types::query::DateBounds;
use menura_model::internal_model::FieldPath;
use menura_representations::bound_query::{
    AuthorizationDecision, AuthorizedSubgraph, BoundDimension, BoundFilter, BoundMetric, BoundPath,
    MetricPath,
};
use menura_representations::columns::Columns;
use menura_representations::origin::QueryLocation;

use crate::named::NamedElement;

/// The model element a reference names.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResolvedName {
    /// A field.
    Field(FieldPath),
    /// A metric.
    Metric(MetricPath),
}

/// A `time_grains` entry whose dimension is resolved and authorized. Grains are matched to
/// the dimensions of the query with rows 10 to 12c (QRY-4.1), so the bound columns carry none
/// yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedTimeGrain {
    /// The element its dimension names.
    pub target: ResolvedName,
    /// The grain.
    pub grain: Grain,
    /// Where it is written.
    pub origin: QueryLocation,
}

/// A date range whose dimension is resolved and authorized; its bounds are as written, and
/// are resolved with rows 10 to 12c.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedDateRange {
    /// The element its dimension names.
    pub target: ResolvedName,
    /// The bounds, as written.
    pub bounds: DateBounds,
    /// Where it is written.
    pub origin: QueryLocation,
}

/// A query after rows 3 to 9 of E2-5: every reference resolves to an authorized model element,
/// every dimension names a field and every metric a metric, every filter reads at most one
/// dataset, and the output names are distinct.
///
/// The ordering terms, the limit and the time settings are those of the query tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedQuery {
    pub(crate) columns: Columns<BoundDimension, BoundMetric>,
    pub(crate) filters: Vec<BoundFilter>,
    pub(crate) time_grains: Vec<AuthorizedTimeGrain>,
    pub(crate) date_range: Option<AuthorizedDateRange>,
    pub(crate) paths: Vec<BoundPath>,
    pub(crate) authorization: Vec<AuthorizationDecision>,
    pub(crate) subgraph: AuthorizedSubgraph,
    pub(crate) named: Vec<NamedElement>,
}

impl AuthorizedQuery {
    /// The output columns, in the order of the query tree. No dimension carries a time grain
    /// yet: see [`AuthorizedQuery::time_grains`].
    #[must_use]
    pub fn columns(&self) -> &Columns<BoundDimension, BoundMetric> {
        &self.columns
    }

    /// The filters, in the order of the query tree, each with the dataset it reads.
    #[must_use]
    pub fn filters(&self) -> &[BoundFilter] {
        &self.filters
    }

    /// The `time_grains` entries, in the order of the query tree.
    #[must_use]
    pub fn time_grains(&self) -> &[AuthorizedTimeGrain] {
        &self.time_grains
    }

    /// The date range, when the query has one.
    #[must_use]
    pub fn date_range(&self) -> Option<&AuthorizedDateRange> {
        self.date_range.as_ref()
    }

    /// The relationships chosen by the query, in the order of the query tree.
    #[must_use]
    pub fn paths(&self) -> &[BoundPath] {
        &self.paths
    }

    /// Every authorization decision, in the order taken (E2-4.7).
    #[must_use]
    pub fn authorization(&self) -> &[AuthorizationDecision] {
        &self.authorization
    }

    /// The authorized subgraph (E2-4.8).
    #[must_use]
    pub fn subgraph(&self) -> &AuthorizedSubgraph {
        &self.subgraph
    }
}

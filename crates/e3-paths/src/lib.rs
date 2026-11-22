//! Stage E3: resolves join paths between datasets (E3-1).
//!
//! [`resolve_paths`] first checks reachability over the authorized subgraph (E3-3, E2-4.8)
//! and chooses the anchor of a dimension-only query (E3-5). It then chooses the path from
//! each home dataset to each needed dataset (E3-3.2, E3-3.3) and builds the branches with
//! their join trees (E3-4), reporting the errors of E2-5 rows 13 to 17 in that order.

#![forbid(unsafe_code)]

mod anchor;
mod components;
mod derived;
mod error;
mod metric_reach;
mod needed;
mod path_choice;
mod reachability;
mod violation;

pub use error::{PathCode, PathDiagnostic, PathError};
pub use violation::InvariantViolation;

use menura_model::internal_model::InternalModel;
use menura_representations::bound_query::BoundQuery;
use menura_representations::paths::{BranchKind, Branches};

use crate::anchor::choose_anchor;
use crate::metric_reach::check_metric_reach;
use crate::needed::NeededDatasets;
use crate::path_choice::{BranchHome, choose_paths, component_homes};
use crate::reachability::AuthorizedGraph;

/// Resolves the branches of `query` against `model` (E3-2).
///
/// Every dimension and filter dataset must be reachable from every metric (E3-3.4 to
/// E3-3.6), through the authorized relationships of the query (E2-4.8) and many-to-one steps
/// only (E3-3.1). Without metrics, the rows come from an anchor (E3-5.1, E3-5.2).
///
/// With metrics, components with the same home dataset form one branch, and the components of
/// each semi-additive metric form their own (E3-4.1). Without
/// metrics, the query is one branch whose home is the anchor, with no components (E3-5.3).
/// Branches are ordered by home dataset, then plain before semi-additive, then by metric name
/// (E3-4.3), and each join tree holds the paths from its
/// home to every needed dataset, one join instance per distinct path prefix (E3-4.2,
/// E3-4.2a).
///
/// # Errors
///
/// [`PathError::Diagnostic`] with the first error of E2-5 rows 13 to 17: `E_NO_PATH`,
/// `E_PARTIAL_DIMENSION_REACHABILITY`, `E_NO_ANCHOR`, `E_UNSUPPORTED_FILTER_PATH`, then
/// `E_AMBIGUOUS_PATH` for more than one anchor or for a path the query must choose, then
/// `E3-01` for a `paths` entry that no branch uses.
///
/// [`PathError::Invariant`] when `query` is one that E2 never lets through: it has neither
/// metrics nor dimensions, has a metric whose components cannot be read from `model`, or
/// names a path that does not resolve in the authorized subgraph.
pub fn resolve_paths(query: &BoundQuery, model: &InternalModel) -> Result<Branches, PathError> {
    if query.columns().first().is_none() {
        return Err(InvariantViolation::NoMetricAndNoDimension.into());
    }
    let needed = NeededDatasets::read(query, model)?;
    let graph = AuthorizedGraph::new(&model.parts().graph, query.subgraph());
    let homes = if needed.metrics.is_empty() {
        vec![BranchHome {
            home: choose_anchor(&needed, &graph)?.clone(),
            kind: BranchKind::Plain,
            components: Vec::new(),
        }]
    } else {
        check_metric_reach(&needed, &graph)?;
        component_homes(query, model)?
    };
    choose_paths(query, model, &needed, homes)
}

#[cfg(test)]
mod reachability_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

//! Stage E3: resolves join paths between datasets (E3-1).
//!
//! [`resolve_paths`] first checks reachability over the authorized subgraph (E3-3, E2-4.8)
//! and chooses the anchor of a dimension-only query (E3-5), reporting the errors of E2-5
//! rows 13 to 17 in that order. It then builds one branch whose join tree is empty, as in the
//! 0.3.0 slice (VS-1): the path from a dataset to itself is empty (E3-3.1). Path choice and
//! join trees among several datasets come with the later steps of 0.4.0.

#![forbid(unsafe_code)]

mod anchor;
mod components;
mod error;
mod metric_reach;
mod needed;
mod path_choice;
mod reachability;
mod single_dataset;
mod violation;

pub use error::{PathCode, PathDiagnostic, PathError};
pub use path_choice::{
    BranchHome, ChoiceInvariant, PathChoiceCode, PathChoiceError, choose_paths, component_homes,
};
pub use violation::InvariantViolation;

use menura_model::internal_model::{InternalModel, NormalizedName};
use menura_representations::bound_query::{BoundDimension, BoundDimensionKind, BoundQuery};
use menura_representations::columns::OutputColumn;
use menura_representations::paths::{Branch, Branches, ComponentKey, JoinTree};

use crate::anchor::choose_anchor;
use crate::components::components;
use crate::metric_reach::check_metric_reach;
use crate::needed::NeededDatasets;
use crate::reachability::AuthorizedGraph;
use crate::single_dataset::SingleDataset;

/// Resolves the branches of `query` against `model` (E3-2).
///
/// Every dimension and filter dataset must be reachable from every metric (E3-3.4 to
/// E3-3.6), through the authorized relationships of the query (E2-4.8) and many-to-one steps
/// only (E3-3.1). Without metrics, the rows come from an anchor (E3-5.1, E3-5.2).
///
/// With metrics, the result is one branch (E3-4.1): its home is the dataset of every
/// component, it holds every component in [`ComponentKey`] order, and its join tree is empty
/// (E3-4.2). Without metrics, it is one branch whose home is the anchor, with no components
/// (E3-5.3).
///
/// # Errors
///
/// [`PathError::Diagnostic`] with the first error of E2-5 rows 13 to 17: `E_NO_PATH`,
/// `E_PARTIAL_DIMENSION_REACHABILITY`, `E_NO_ANCHOR`, `E_UNSUPPORTED_FILTER_PATH`, then
/// `E_AMBIGUOUS_PATH` for more than one anchor.
///
/// [`PathError::Invariant`] when `query` is outside what E3 resolves today, which E2 never lets
/// through (E2-5 row 12c): it has neither metrics nor dimensions, has a metric whose
/// components cannot be read from `model`, chooses a path, or reads more than one dataset.
pub fn resolve_paths(query: &BoundQuery, model: &InternalModel) -> Result<Branches, PathError> {
    if query.columns().first().is_none() {
        return Err(InvariantViolation::NoMetricAndNoDimension.into());
    }
    let needed = NeededDatasets::read(query, model)?;
    let graph = AuthorizedGraph::new(&model.parts().graph, query.subgraph());
    let anchor = if needed.metrics.is_empty() {
        Some(choose_anchor(&needed, &graph)?.clone())
    } else {
        check_metric_reach(&needed, &graph)?;
        None
    };
    single_dataset_branches(query, model, anchor)
}

/// The one branch of a query that reads one dataset (VS-1): its home is `anchor` for a
/// dimension-only query, and the dataset of its components otherwise.
fn single_dataset_branches(
    query: &BoundQuery,
    model: &InternalModel,
    anchor: Option<NormalizedName>,
) -> Result<Branches, PathError> {
    if let Some(path) = query.paths().first() {
        return Err(InvariantViolation::ChosenPath {
            origin: path.origin().clone(),
        }
        .into());
    }
    let mut dataset = SingleDataset::default();
    let keys = read_columns(query, model, &mut dataset)?;
    read_filters(query, &mut dataset)?;
    let Some(home) = dataset.into_dataset() else {
        return Err(InvariantViolation::NoMetricAndNoDimension.into());
    };
    if let Some(anchor) = anchor {
        return Ok(Branches::dimension_only(anchor, JoinTree::default()));
    }
    let branch = Branch {
        home,
        components: keys,
        join_tree: JoinTree::default(),
    };
    Branches::new(vec![branch]).ok_or_else(|| InvariantViolation::DuplicateHomeDataset.into())
}

/// Reads the dataset of every output column, in output order, and returns the key of every
/// metric component, in [`ComponentKey`] order.
fn read_columns(
    query: &BoundQuery,
    model: &InternalModel,
    dataset: &mut SingleDataset,
) -> Result<Vec<ComponentKey>, InvariantViolation> {
    let mut keys = Vec::new();
    let mut metric_position = 0;
    for column in query.columns().iter() {
        match column {
            OutputColumn::Dimension(dimension) => read_dimension(dimension, dataset)?,
            OutputColumn::Metric(metric) => {
                for (position, component) in components(metric, model)?.iter().enumerate() {
                    dataset.read(&component.home_dataset, &metric.origin)?;
                    keys.push(ComponentKey {
                        metric: metric_position,
                        component: position,
                    });
                }
                metric_position += 1;
            }
        }
    }
    Ok(keys)
}

/// Reads the dataset of `dimension`.
fn read_dimension(
    dimension: &BoundDimension,
    dataset: &mut SingleDataset,
) -> Result<(), InvariantViolation> {
    let read = match &dimension.kind {
        BoundDimensionKind::Field { field, .. } => &field.dataset,
        BoundDimensionKind::Expression { dataset, .. } => dataset,
    };
    dataset.read(read, &dimension.origin)
}

/// Reads the datasets of the filters, then the dataset of the date range field, in query
/// order (E2-5.3). A filter that reads no dataset reads nothing here.
fn read_filters(query: &BoundQuery, dataset: &mut SingleDataset) -> Result<(), InvariantViolation> {
    for filter in query.filters() {
        for read in &filter.datasets {
            dataset.read(read, &filter.origin)?;
        }
    }
    if let Some(date_range) = query.date_range() {
        dataset.read(&date_range.field.dataset, &date_range.origin)?;
    }
    Ok(())
}

#[cfg(test)]
mod reachability_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

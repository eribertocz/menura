//! Stage E3: resolves join paths between datasets (E3-1).
//!
//! In the 0.3.0 slice (VS-1) every element of a query belongs to one dataset, and the query
//! names no `paths`. [`resolve_paths`] therefore builds one branch whose join tree is empty:
//! the path from a dataset to itself is empty (E3-3.1). Paths, reachability and anchors among
//! several datasets are added in 0.4.0.

#![forbid(unsafe_code)]

mod components;
mod path_choice;
mod single_dataset;
mod violation;

pub use path_choice::{
    BranchHome, ChoiceInvariant, PathChoiceCode, PathChoiceError, choose_paths, component_homes,
};
pub use violation::InvariantViolation;

use menura_model::internal_model::InternalModel;
use menura_representations::bound_query::{BoundDimension, BoundDimensionKind, BoundQuery};
use menura_representations::columns::OutputColumn;
use menura_representations::paths::{Branch, Branches, ComponentKey, JoinTree};

use crate::components::components;
use crate::single_dataset::SingleDataset;

/// Resolves the branches of `query` against `model` (E3-2).
///
/// With metrics, the result is one branch (E3-4.1): its home is the dataset of every
/// component, it holds every component in [`ComponentKey`] order, and its join tree is empty
/// (E3-4.2). Without metrics, it is one branch whose home is the dataset of the dimensions,
/// with no components (E3-5.3).
///
/// # Errors
///
/// An [`InvariantViolation`] when `query` is outside the 0.3.0 slice, which E2 never lets
/// through (E2-5 row 12c): it chooses a path, has neither metrics nor dimensions, reads more
/// than one dataset, or has a metric whose components cannot be read from `model`.
pub fn resolve_paths(
    query: &BoundQuery,
    model: &InternalModel,
) -> Result<Branches, InvariantViolation> {
    if let Some(path) = query.paths().first() {
        return Err(InvariantViolation::ChosenPath {
            origin: path.origin().clone(),
        });
    }
    if query.columns().first().is_none() {
        return Err(InvariantViolation::NoMetricAndNoDimension);
    }
    let mut dataset = SingleDataset::default();
    let keys = read_columns(query, model, &mut dataset)?;
    read_filters(query, &mut dataset)?;
    let Some(home) = dataset.into_dataset() else {
        return Err(InvariantViolation::NoMetricAndNoDimension);
    };
    if keys.is_empty() {
        return Ok(Branches::dimension_only(home, JoinTree::default()));
    }
    let branch = Branch {
        home,
        components: keys,
        join_tree: JoinTree::default(),
    };
    Branches::new(vec![branch]).ok_or(InvariantViolation::DuplicateHomeDataset)
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
mod test_support;
#[cfg(test)]
mod tests;

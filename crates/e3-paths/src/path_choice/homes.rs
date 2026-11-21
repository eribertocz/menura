//! The home datasets of the components of a query (E3-4.1, E3-4.3).

use std::collections::BTreeMap;

use menura_model::internal_model::{InternalModel, NormalizedName};
use menura_representations::bound_query::BoundQuery;
use menura_representations::columns::OutputColumn;
use menura_representations::paths::ComponentKey;

use crate::components::components;
use crate::violation::InvariantViolation;

/// A home dataset with the components evaluated over it: the start of one branch (E3-4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BranchHome {
    /// The home dataset, or the anchor of a dimension-only query (E3-5.3).
    pub(crate) home: NormalizedName,
    /// Its components, in [`ComponentKey`] order; empty for a dimension-only query.
    pub(crate) components: Vec<ComponentKey>,
}

/// The home datasets of the components of `query`, ordered by name (E3-4.3), each with its
/// components in [`ComponentKey`] order (E3-4.1). Empty when the query has no metrics.
///
/// # Errors
///
/// An [`InvariantViolation`] for a metric whose components cannot be read from `model`,
/// which E2 never lets through.
pub(crate) fn component_homes(
    query: &BoundQuery,
    model: &InternalModel,
) -> Result<Vec<BranchHome>, InvariantViolation> {
    let mut homes: BTreeMap<&NormalizedName, Vec<ComponentKey>> = BTreeMap::new();
    let metrics = query.columns().iter().filter_map(|column| match column {
        OutputColumn::Metric(metric) => Some(metric),
        OutputColumn::Dimension(_) => None,
    });
    for (metric_position, metric) in metrics.enumerate() {
        for (position, component) in components(metric, model)?.iter().enumerate() {
            homes
                .entry(&component.home_dataset)
                .or_default()
                .push(ComponentKey {
                    metric: metric_position,
                    component: position,
                });
        }
    }
    Ok(homes
        .into_iter()
        .map(|(home, components)| BranchHome {
            home: home.clone(),
            components,
        })
        .collect())
}

//! The home datasets of the components of a query (E3-4.1, E3-4.3).

use std::collections::BTreeMap;

use menura_model::internal_model::{DeclaredAdditivity, InternalModel, Metric, NormalizedName};
use menura_representations::bound_query::BoundQuery;
use menura_representations::columns::OutputColumn;
use menura_representations::paths::{BranchKind, ComponentKey};

use crate::components::components;
use crate::violation::InvariantViolation;

/// A home dataset with the components evaluated over it: the start of one branch (E3-4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BranchHome {
    /// The home dataset, or the anchor of a dimension-only query (E3-5.3).
    pub(crate) home: NormalizedName,
    /// Which components the branch holds.
    pub(crate) kind: BranchKind,
    /// Its components, in [`ComponentKey`] order; empty for a dimension-only query.
    pub(crate) components: Vec<ComponentKey>,
}

/// The branch starts of the components of `query`, in the order of E3-4.3, each with its
/// components in [`ComponentKey`] order: one per home dataset for the components of metrics
/// that are not semi-additive, and one per home dataset and semi-additive metric (E3-4.1).
/// Empty when the query has no metrics.
///
/// # Errors
///
/// An [`InvariantViolation`] for a metric whose components cannot be read from `model`,
/// which E2 never lets through.
pub(crate) fn component_homes(
    query: &BoundQuery,
    model: &InternalModel,
) -> Result<Vec<BranchHome>, InvariantViolation> {
    let mut homes: BTreeMap<(&NormalizedName, BranchKind), Vec<ComponentKey>> = BTreeMap::new();
    let metrics = query.columns().iter().filter_map(|column| match column {
        OutputColumn::Metric(metric) => Some(metric),
        OutputColumn::Dimension(_) => None,
    });
    for (metric_position, metric) in metrics.enumerate() {
        for (position, base) in components(metric, model)?.into_iter().enumerate() {
            homes
                .entry((&base.component.home_dataset, branch_kind(base.metric)))
                .or_default()
                .push(ComponentKey {
                    metric: metric_position,
                    component: position,
                });
        }
    }
    Ok(homes
        .into_iter()
        .map(|((home, kind), components)| BranchHome {
            home: home.clone(),
            kind,
            components,
        })
        .collect())
}

/// The kind of the branch of a component of the base metric `metric`: its own branch for a
/// semi-additive metric (E3-4.1); a `non_additive` metric plans as its deduced additivity
/// (E4-7.9).
fn branch_kind(metric: &Metric) -> BranchKind {
    match metric.declared_additivity {
        Some(DeclaredAdditivity::SemiAdditive(_)) => BranchKind::SemiAdditive {
            metric: metric.name.clone(),
        },
        Some(DeclaredAdditivity::NonAdditive) | None => BranchKind::Plain,
    }
}

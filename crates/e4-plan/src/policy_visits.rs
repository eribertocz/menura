//! The row filters of every dataset whose policies take part in the visible rows of the home
//! dataset, checked in the visit order of E4-6.9: `ESEC-02` over every dataset first, then
//! `EQRY-03` (E4-6.9b, E2-5 row 18).

use std::collections::BTreeMap;

use menura_model::expression::Expression;
use menura_model::internal_model::{Dataset, InternalModel, NormalizedName, SourceLocation};
use menura_representations::origin::Origin;
use menura_security::{AdmittedSecurityContext, RowFilterError, row_filter};

use crate::error::{InvariantViolation, PlanError};
use crate::followed::PolicyGraph;
use crate::policy::reads_now;

/// The row filter `G AND R` of one dataset for the user, as E0 wrote it, with the location of
/// its first policy.
#[derive(Debug, Clone)]
pub(crate) struct RowFilter {
    /// The filter, over the fields of its dataset.
    pub(crate) filter: Expression,
    /// Where the first policy of the dataset is declared.
    pub(crate) location: SourceLocation,
}

/// The row filters of the datasets that have row policies, by dataset.
#[derive(Debug, Clone, Default)]
pub(crate) struct RowFilters(BTreeMap<NormalizedName, RowFilter>);

impl RowFilters {
    /// The row filter of `dataset`; `None` when it has no policy.
    pub(crate) fn of(&self, dataset: &NormalizedName) -> Option<&RowFilter> {
        self.0.get(dataset)
    }
}

/// The row filters of `home` and of every dataset it reaches that has row policies, for the
/// user of `context`.
///
/// # Errors
///
/// [`PlanError::MissingAttribute`] (`ESEC-02`) for the first applicable policy, in visit order,
/// that needs an absent attribute; then [`PlanError::MissingReferenceTime`] (`EQRY-03`) for the
/// first one that reads the current date or time without `reference_time`; an internal error
/// for a condition E0 did not parse.
pub(crate) fn row_filters<'model>(
    home: &'model Dataset,
    model: &'model InternalModel,
    graph: &PolicyGraph<'model>,
    context: &AdmittedSecurityContext,
    has_reference_time: bool,
) -> Result<RowFilters, PlanError> {
    row_filters_from(&[home], model, graph, context, has_reference_time)
}

/// The row filters of each dataset of `roots` and of every dataset it reaches that has row
/// policies, for the user of `context`, visited root after root: the datasets of each root
/// in [`visit_order`], skipping a dataset an earlier root already visited (E4-6.9).
///
/// # Errors
///
/// As [`row_filters`].
pub(crate) fn row_filters_from<'model>(
    roots: &[&'model Dataset],
    model: &'model InternalModel,
    graph: &PolicyGraph<'model>,
    context: &AdmittedSecurityContext,
    has_reference_time: bool,
) -> Result<RowFilters, PlanError> {
    let mut visited: Vec<&NormalizedName> = Vec::new();
    let mut order = Vec::new();
    for root in roots {
        for dataset in visit_order(root, model, graph) {
            if !visited.contains(&&dataset.name) {
                visited.push(&dataset.name);
                order.push(dataset);
            }
        }
    }
    let mut filters = BTreeMap::new();
    for dataset in order {
        let filter = row_filter(dataset, context).map_err(|error| match error {
            RowFilterError::MissingAttribute { .. } => PlanError::MissingAttribute(error),
            RowFilterError::UnparsedCondition { location } => PlanError::invariant(
                InvariantViolation::UnparsedCondition,
                &Origin::Model(location),
            ),
        })?;
        if let (Some(filter), Some(first)) = (filter, dataset.row_policies.first()) {
            filters.insert(
                dataset.name.clone(),
                RowFilter {
                    filter,
                    location: first.location.clone(),
                },
            );
        }
    }
    if !has_reference_time
        && filters
            .values()
            .any(|row_filter: &RowFilter| reads_now(&row_filter.filter))
    {
        return Err(PlanError::MissingReferenceTime);
    }
    Ok(RowFilters(filters))
}

/// The datasets with row policies that take part in the visible rows of `home`: `home` first,
/// then the others by the length of their shortest path from `home`, then by name; each once.
fn visit_order<'model>(
    home: &'model Dataset,
    model: &'model InternalModel,
    graph: &PolicyGraph<'model>,
) -> Vec<&'model Dataset> {
    let mut reached: Vec<(usize, &NormalizedName)> = graph
        .distances(&home.name)
        .into_iter()
        .filter(|(dataset, _)| *dataset != &home.name)
        .map(|(dataset, distance)| (distance, dataset))
        .collect();
    reached.sort();
    let mut order = vec![home];
    order.extend(
        reached
            .into_iter()
            .filter_map(|(_, dataset)| model.dataset(dataset))
            .filter(|dataset| !dataset.row_policies.is_empty()),
    );
    order
}

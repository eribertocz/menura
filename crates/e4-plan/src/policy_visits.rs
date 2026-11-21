//! The row filters of every dataset whose policies take part in the visible rows of the home
//! dataset, checked in the visit order of E4-6.9: `ESEC-02` over every dataset first, then
//! `EQRY-03` (E4-6.9b, E2-5 row 18).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use menura_model::expression::Expression;
use menura_model::internal_model::{Dataset, InternalModel, NormalizedName, SourceLocation};
use menura_representations::origin::Origin;
use menura_security::{AdmittedSecurityContext, RowFilterError, row_filter};

use crate::error::{InvariantViolation, PlanError};
use crate::followed::has_policies;
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
pub(crate) fn row_filters(
    home: &Dataset,
    model: &InternalModel,
    context: &AdmittedSecurityContext,
    has_reference_time: bool,
) -> Result<RowFilters, PlanError> {
    let mut filters = BTreeMap::new();
    for dataset in visit_order(home, model) {
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
) -> Vec<&'model Dataset> {
    let mut order = vec![home];
    let mut reached: Vec<(usize, &NormalizedName)> = distances(model, &home.name)
        .into_iter()
        .filter(|(dataset, _)| *dataset != &home.name && has_policies(model, dataset))
        .map(|(dataset, distance)| (distance, dataset))
        .collect();
    reached.sort();
    order.extend(
        reached
            .into_iter()
            .filter_map(|(_, dataset)| model.dataset(dataset)),
    );
    order
}

/// The length of the shortest path from `home` to every dataset it reaches along usable
/// relationships.
fn distances<'model>(
    model: &'model InternalModel,
    home: &'model NormalizedName,
) -> Vec<(&'model NormalizedName, usize)> {
    let edges = &model.parts().graph.edges;
    let mut seen: BTreeSet<&NormalizedName> = BTreeSet::from([home]);
    let mut found = Vec::new();
    let mut pending = VecDeque::from([(home, 0)]);
    while let Some((current, distance)) = pending.pop_front() {
        found.push((current, distance));
        for edge in edges.iter().filter(|edge| &edge.from == current) {
            if seen.insert(&edge.to) {
                pending.push_back((&edge.to, distance + 1));
            }
        }
    }
    found
}

//! The row-policy checks of the whole query (E4-6.9, E4-6.9b, E2-5 row 18): every `ESEC-02`
//! check over the visit order of every branch, populations included, before any `EQRY-03`
//! check.

use menura_model::internal_model::{Dataset, InternalModel};
use menura_representations::paths::{BranchKind, Branches};
use menura_security::AdmittedSecurityContext;

use crate::error::PlanError;
use crate::followed::PolicyGraph;
use crate::metrics::{PlannedMetric, semi_additive_metric};
use crate::policy_visits::row_filters_from;
use crate::population::declared;
use crate::slice::home_dataset;

/// Checks the row policies of `branches` for the user of `security`: branch after branch, the
/// datasets of each home dataset's visible rows, then, for a semi-additive branch with a
/// population, those of the dataset that declares it; each dataset once per query (E4-6.9).
///
/// # Errors
///
/// [`PlanError::MissingAttribute`] (`ESEC-02`) for the first applicable policy in that order
/// that needs an absent attribute; then [`PlanError::MissingReferenceTime`] (`EQRY-03`) when
/// one reads the current date or time and the query has no `reference_time`; an internal
/// error for an input the stages before E4 never produce.
pub(crate) fn check_query_policies(
    branches: &Branches,
    metrics: &[PlannedMetric<'_>],
    model: &InternalModel,
    security: &AdmittedSecurityContext,
    has_reference_time: bool,
) -> Result<(), PlanError> {
    let roots = visit_roots(branches, metrics, model)?;
    let graph = PolicyGraph::new(model);
    row_filters_from(&roots, model, &graph, security, has_reference_time).map(|_| ())
}

/// The datasets whose visible rows the query reads, in visit order: each branch's home
/// dataset, then the dataset of its population when it has one.
pub(crate) fn visit_roots<'model>(
    branches: &Branches,
    metrics: &[PlannedMetric<'model>],
    model: &'model InternalModel,
) -> Result<Vec<&'model Dataset>, PlanError> {
    let mut roots = Vec::new();
    for branch in branches.as_slice() {
        roots.push(home_dataset(branch, model)?);
        if let BranchKind::SemiAdditive { .. } = branch.kind {
            let (_, declaration) = semi_additive_metric(branch, metrics)?;
            if let Some(population) = &declaration.population {
                roots.push(declared(model, population)?.0);
            }
        }
    }
    Ok(roots)
}

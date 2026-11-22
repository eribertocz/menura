//! E5-3.5, E5-3.5a, E5-4: selects a summary for each branch and reads it instead of the home
//! dataset.
//!
//! [`eligibility`] checks the conditions of SUM-3 that need no catalog state, `freshness` the
//! conditions of CAT-7.2, and [`summary_branch`] builds the branch that reads an eligible
//! summary (E5-3.5a). `selection` chooses among the valid summaries of each branch (E5-4).

mod branch;
mod calendar;
pub(crate) mod condition;
pub(crate) mod eligibility;
mod equivalence;
mod exclusions;
mod expression_map;
mod freshness;
#[cfg(test)]
mod freshness_build_tests;
#[cfg(test)]
mod freshness_fixture;
#[cfg(test)]
mod freshness_tests;
mod period;
#[cfg(test)]
mod period_tests;
mod reads;
mod restricted;
mod selection;
pub(crate) mod summary_branch;
mod walk;

#[cfg(test)]
mod tests;

use menura_representations::logical_plan::PlanNode;

use crate::context::RewriteContext;
use crate::record::RewriteRecord;

/// Applies the rewrite to `plan`: each branch reads the summary E5-4 selects for it, if any.
/// Adds a record of every summary considered for a branch to `records`.
pub(super) fn rewrite(
    plan: PlanNode,
    context: &RewriteContext<'_>,
    records: &mut Vec<RewriteRecord>,
) -> PlanNode {
    walk::select_in_branches(&plan, context, records).unwrap_or(plan)
}

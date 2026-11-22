//! E5-3.5, E5-3.5a, E5-4: selects a summary for each branch and reads it instead of the home dataset.

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

use menura_representations::logical_plan::PlanNode;

use crate::context::RewriteContext;
use crate::record::RewriteRecord;

/// Applies the rewrite to `plan` and adds a record of every case it considered to `records`.
/// Not yet implemented: returns `plan` unchanged and records nothing.
pub(super) fn rewrite(
    plan: PlanNode,
    _context: &RewriteContext<'_>,
    _records: &mut Vec<RewriteRecord>,
) -> PlanNode {
    plan
}

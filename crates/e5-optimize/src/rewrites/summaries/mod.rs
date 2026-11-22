//! E5-3.5, E5-3.5a, E5-4: selects a summary for each branch and reads it instead of the home
//! dataset.
//!
//! [`eligibility`] checks the conditions of SUM-3 that need no catalog state, and
//! [`summary_branch`] builds the branch that reads an eligible summary (E5-3.5a). The
//! selection of E5-4 calls both; until it does, the rewrite leaves the plan unchanged.

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "called by the summary selection of E5-4 (#397), which is not written yet"
    )
)]
mod branch;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "called by the summary selection of E5-4 (#397), which is not written yet"
    )
)]
mod calendar;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "called by the summary selection of E5-4 (#397), which is not written yet"
    )
)]
pub(crate) mod condition;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "called by the summary selection of E5-4 (#397), which is not written yet"
    )
)]
pub(crate) mod eligibility;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "called by the summary selection of E5-4 (#397), which is not written yet"
    )
)]
mod equivalence;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "called by the summary selection of E5-4 (#397), which is not written yet"
    )
)]
mod exclusions;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "called by the summary selection of E5-4 (#397), which is not written yet"
    )
)]
mod expression_map;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "called by the summary selection of E5-4 (#397), which is not written yet"
    )
)]
mod reads;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "called by the summary selection of E5-4 (#397), which is not written yet"
    )
)]
mod restricted;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "called by the summary selection of E5-4 (#397), which is not written yet"
    )
)]
pub(crate) mod summary_branch;

#[cfg(test)]
mod tests;

use menura_representations::logical_plan::PlanNode;

use crate::context::RewriteContext;
use crate::record::RewriteRecord;

/// Applies the rewrite to `plan` and adds a record of every case it considered to `records`.
/// Summary selection (E5-4) is not yet implemented: returns `plan` unchanged and records
/// nothing.
pub(super) fn rewrite(
    plan: PlanNode,
    _context: &RewriteContext<'_>,
    _records: &mut Vec<RewriteRecord>,
) -> PlanNode {
    plan
}

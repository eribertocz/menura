//! E5-3.1, E5-3.1a: removes the `Join` nodes whose dataset contributes no column, filter or policy.

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

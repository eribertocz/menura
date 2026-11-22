//! The rewrites of E5-3, one module each, in the order they run.

mod deduplication;
mod fold_constants;
mod inner_joins;
mod merge_scans;
mod summaries;
mod unused_joins;

#[cfg(test)]
mod deduplication_tests;

use menura_representations::logical_plan::PlanNode;

use crate::context::RewriteContext;
use crate::record::{RewriteRecord, RewriteRule};

/// A rewrite: takes the plan, returns the rewritten plan and records what it considered.
pub(crate) type Rewrite = fn(PlanNode, &RewriteContext<'_>, &mut Vec<RewriteRecord>) -> PlanNode;

/// The rewrites in the order of E5-3; each runs on the output of the previous one. Summary
/// selection (E5-4) runs inside `summaries`, after E5-3.1 to E5-3.4 and before E5-3.6.
pub(crate) const PIPELINE: [(RewriteRule, Rewrite); 6] = [
    (RewriteRule::RemoveUnusedJoins, unused_joins::rewrite),
    (RewriteRule::SkipDeduplication, deduplication::rewrite),
    (RewriteRule::InnerJoins, inner_joins::rewrite),
    (RewriteRule::MergeScans, merge_scans::rewrite),
    (RewriteRule::UseSummary, summaries::rewrite),
    (RewriteRule::FoldConstants, fold_constants::rewrite),
];

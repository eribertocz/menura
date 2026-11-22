//! E5-4: the summary each branch reads, among those that meet every condition of E5-3.5.

use menura_contracts::types::catalog::CatalogStateDocument;
use menura_model::internal_model::{NormalizedName, Summary};
use menura_representations::bound_query::BoundDateRange;
use menura_representations::logical_plan::PlanNode;

use super::branch::BranchShape;
use super::condition::SummaryCondition;
use super::eligibility::{Eligibility, eligibility};
use super::freshness::{FailedCondition, check_freshness};
use super::summary_branch::summary_branch;
use crate::context::RewriteContext;
use crate::record::{RewriteOutcome, RewriteRecord, RewriteRule, RewriteSubject};

/// The failed condition recorded for a valid summary that another one was chosen over.
const NOT_CHOSEN: &str = "E5-4 step 2";

/// A summary that meets every condition of E5-3.5 for a branch (E5-4 step 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Candidate {
    /// The summary dataset.
    pub(super) summary: NormalizedName,
    /// The `row_count_estimate` of its relation in the catalog state.
    pub(super) row_count_estimate: u64,
    /// The branch that reads it (E5-3.5a).
    pub(super) branch: PlanNode,
}

/// The candidate with the smallest `row_count_estimate`, ties broken by the normalized name of
/// the summary (E5-4 step 2, P3); `None` when there is none.
pub(super) fn smallest<'a>(
    candidates: impl IntoIterator<Item = &'a Candidate>,
) -> Option<&'a Candidate> {
    candidates.into_iter().min_by(|left, right| {
        (left.row_count_estimate, left.summary.as_str())
            .cmp(&(right.row_count_estimate, right.summary.as_str()))
    })
}

/// The branch `branch` reading the summary E5-4 selects for it, or `None` when no summary of
/// its home dataset meets every condition of E5-3.5 (E5-4 steps 1 to 3).
///
/// Adds one record per summary of the home dataset, in the order of their names: applied for
/// the selected one, and otherwise rejected with the first condition it fails (a rule ID of
/// SUM-3, or `CAT-7.2 condition N` for freshness), or with `E5-4 step 2` when another valid
/// summary was chosen. A branch that is not an `Aggregate` over the nodes of E4-4 has no home
/// dataset to read summaries of, and gets no record.
pub(super) fn select(
    branch: &PlanNode,
    context: &RewriteContext<'_>,
    records: &mut Vec<RewriteRecord>,
) -> Option<PlanNode> {
    let home = BranchShape::of(branch)?.home.clone();
    let considered: Vec<(&Summary, Result<Candidate, String>)> = context
        .model
        .parts()
        .summaries
        .iter()
        .filter(|summary| summary.of == home)
        .map(|summary| (summary, candidate(branch, summary, context)))
        .collect();
    let chosen = smallest(
        considered
            .iter()
            .filter_map(|(_, result)| result.as_ref().ok()),
    )
    .map(|candidate| candidate.summary.clone());
    for (summary, result) in &considered {
        let outcome = match result {
            Err(failed_condition) => rejected(failed_condition),
            Ok(candidate) if chosen.as_ref() == Some(&candidate.summary) => RewriteOutcome::Applied,
            Ok(_) => rejected(NOT_CHOSEN),
        };
        records.push(RewriteRecord {
            rule: RewriteRule::UseSummary,
            subject: RewriteSubject::Summary(summary.dataset.clone()),
            outcome,
        });
    }
    let chosen = chosen?;
    considered
        .into_iter()
        .filter_map(|(_, result)| result.ok())
        .find(|candidate| candidate.summary == chosen)
        .map(|candidate| candidate.branch)
}

/// `summary` as a candidate for `branch`, or the first condition of E5-3.5 it fails, in the
/// order of SUM-3: SUM-3.1 to SUM-3.6, then freshness (SUM-3.7, CAT-7.2), then SUM-3.8.
fn candidate(
    branch: &PlanNode,
    summary: &Summary,
    context: &RewriteContext<'_>,
) -> Result<Candidate, String> {
    let date_range = BranchShape::of(branch).and_then(|shape| shape.date_range(context.query));
    let date_range = date_range.as_ref();
    let summary_use = match eligibility(branch, summary, context) {
        Eligibility::Eligible(summary_use) => summary_use,
        Eligibility::Ineligible(SummaryCondition::SameType) => {
            fresh_state(summary, context, date_range)?;
            return Err(SummaryCondition::SameType.rule_id().to_owned());
        }
        Eligibility::Ineligible(condition) => return Err(condition.rule_id().to_owned()),
    };
    let state = fresh_state(summary, context, date_range)?;
    let row_count_estimate = row_count_estimate(summary, context, state)
        .ok_or_else(|| freshness_condition(FailedCondition::Coverage))?;
    let branch = summary_branch(branch, &summary_use)
        .ok_or_else(|| SummaryCondition::Grain.rule_id().to_owned())?;
    Ok(Candidate {
        summary: summary_use.summary().clone(),
        row_count_estimate,
        branch,
    })
}

/// The catalog state when it proves `summary` fresh for the query (SUM-3.7, CAT-7.2), or the
/// first condition that fails, over `date_range`, the range the branch reads (E4-8.9). With no
/// state, the partitions of condition 1 are missing.
fn fresh_state<'a>(
    summary: &Summary,
    context: &RewriteContext<'a>,
    date_range: Option<&BoundDateRange>,
) -> Result<&'a CatalogStateDocument, String> {
    let state = context
        .evidence
        .state()
        .ok_or_else(|| freshness_condition(FailedCondition::Partitions))?;
    check_freshness(
        summary,
        context.model,
        context.schema,
        state,
        &context.query.time().time_zone,
        date_range,
    )
    .map_err(freshness_condition)?;
    Ok(state)
}

/// The `row_count_estimate` of the relation of `summary` in `state`.
fn row_count_estimate(
    summary: &Summary,
    context: &RewriteContext<'_>,
    state: &CatalogStateDocument,
) -> Option<u64> {
    let source = &context.model.dataset(&summary.dataset)?.source;
    state
        .relations
        .iter()
        .find(|relation| &relation.source == source)
        .map(|relation| relation.row_count_estimate)
}

/// The failed freshness condition as a record states it: `CAT-7.2 condition N`.
fn freshness_condition(condition: FailedCondition) -> String {
    let number = match condition {
        FailedCondition::Partitions => 1,
        FailedCondition::Coverage => 2,
        FailedCondition::BuiltFrom => 3,
        FailedCondition::DefinitionHash => 4,
        FailedCondition::TimeZone => 5,
        FailedCondition::IntegrityCheck => 6,
    };
    format!("CAT-7.2 condition {number}")
}

fn rejected(failed_condition: &str) -> RewriteOutcome {
    RewriteOutcome::Rejected {
        failed_condition: failed_condition.to_owned(),
    }
}

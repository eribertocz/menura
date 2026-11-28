//! The E5 entries of the explanation (E5-2.2, E5-3.8, PLN-7.3): one per rewrite record, then
//! one per summary used, which requires the authorized executor (E5-4 step 5).

use menura_contracts::types::common::Step;
use menura_model::explanation::{ExplanationEntry, Subject};
use menura_representations::logical_plan::{Operation, PlanNode};

use crate::plan_inputs::home_dataset;
use crate::record::{RewriteOutcome, RewriteRecord, RewriteRule, RewriteSubject};

/// The entries of `records`, made by one rewrite whose input plan is `input`, in record
/// order. A record whose subject `input` does not have gets no entry.
pub(crate) fn rewrite_entries(
    records: &[RewriteRecord],
    input: &PlanNode,
) -> Vec<ExplanationEntry> {
    records
        .iter()
        .filter_map(|record| {
            Some(ExplanationEntry {
                code: record.rule.rule_id(),
                step: Step::E5,
                subject: subject(record, input)?,
                decision: decision(&record.outcome).to_owned(),
                reason: reason(record, input),
            })
        })
        .collect()
}

/// One `E5-4` entry per summary that `records` show used, in record order (E5-4 step 5,
/// SUM-4.2).
pub(crate) fn authorized_executor_entries(records: &[RewriteRecord]) -> Vec<ExplanationEntry> {
    records
        .iter()
        .filter_map(
            |record| match (&record.rule, &record.subject, &record.outcome) {
                (
                    RewriteRule::UseSummary,
                    RewriteSubject::Summary(summary),
                    RewriteOutcome::Applied,
                ) => Some(ExplanationEntry {
                    code: "E5-4",
                    step: Step::E5,
                    subject: Subject::Summary(summary.clone()),
                    decision: "requires authorized executor".to_owned(),
                    reason: "a branch reads the summary, which is access: internal, so only the \
                             authorized executor may read its relation"
                        .to_owned(),
                }),
                _ => None,
            },
        )
        .collect()
}

/// The subject of `record` (E5-3.8): the relationship of the `Join` it concerns, the home
/// dataset of the branch, or the summary.
fn subject(record: &RewriteRecord, input: &PlanNode) -> Option<Subject> {
    let node = match &record.subject {
        RewriteSubject::Summary(summary) => return Some(Subject::Summary(summary.clone())),
        RewriteSubject::Node(path) => node_at(input, path.steps())?,
    };
    match record.rule {
        RewriteRule::RemoveUnusedJoins | RewriteRule::InnerJoins => match node.operation() {
            Operation::Join { relationship, .. } => {
                Some(Subject::Relationship(relationship.clone()))
            }
            _ => None,
        },
        RewriteRule::SkipDeduplication | RewriteRule::MergeScans | RewriteRule::FoldConstants => {
            home_dataset(node).cloned().map(Subject::Dataset)
        }
        RewriteRule::UseSummary => None,
    }
}

/// The node of `plan` at `steps`: the input at each index of `PlanNode::inputs`, from the root.
fn node_at<'plan>(plan: &'plan PlanNode, steps: &[usize]) -> Option<&'plan PlanNode> {
    steps
        .iter()
        .try_fold(plan, |node, index| node.inputs().get(*index).copied())
}

/// `applied` or `rejected` (PLN-7).
fn decision(outcome: &RewriteOutcome) -> &'static str {
    match outcome {
        RewriteOutcome::Applied => "applied",
        RewriteOutcome::Rejected { .. } => "rejected",
    }
}

/// The condition that held or failed.
fn reason(record: &RewriteRecord, input: &PlanNode) -> String {
    match &record.outcome {
        RewriteOutcome::Applied if is_temporal(record, input) => {
            temporal_condition(record.rule).to_owned()
        }
        RewriteOutcome::Applied => condition(record.rule).to_owned(),
        RewriteOutcome::Rejected { failed_condition } => {
            format!("condition not met: {failed_condition}")
        }
    }
}

/// Whether the subject of `record` is a `Join` along a temporal relationship, or a branch with
/// one, whose evidence is that of VAL-2.14 and VAL-2.15.
fn is_temporal(record: &RewriteRecord, input: &PlanNode) -> bool {
    let RewriteSubject::Node(path) = &record.subject else {
        return false;
    };
    let Some(node) = node_at(input, path.steps()) else {
        return false;
    };
    match record.rule {
        RewriteRule::InnerJoins => matches!(
            node.operation(),
            Operation::Join {
                temporal: Some(_),
                ..
            }
        ),
        RewriteRule::SkipDeduplication => joins_temporally(node),
        _ => false,
    }
}

/// Whether `node` or a node under it is a `Join` along a temporal relationship.
fn joins_temporally(node: &PlanNode) -> bool {
    matches!(
        node.operation(),
        Operation::Join {
            temporal: Some(_),
            ..
        }
    ) || node.inputs().into_iter().any(joins_temporally)
}

/// The condition of `rule` that held when it was applied to a temporal relationship (VAL-2.14,
/// VAL-2.15); the same as [`condition`] for the other rules.
fn temporal_condition(rule: RewriteRule) -> &'static str {
    match rule {
        RewriteRule::SkipDeduplication => {
            "passing key_uniqueness evidence for every plain relationship, and population evidence \
             for the population of every temporal relationship, that caused the Deduplicate"
        }
        RewriteRule::InnerJoins => {
            "passing temporal_orphans evidence, no NULL value possible in from_columns and \
             from_time, and every row of both sides reaches the join"
        }
        other => condition(other),
    }
}

/// The condition of `rule` that held when it was applied (E5-3).
fn condition(rule: RewriteRule) -> &'static str {
    match rule {
        RewriteRule::RemoveUnusedJoins => "no later node reads a column of the joined dataset",
        RewriteRule::SkipDeduplication => {
            "passing key_uniqueness evidence for every relationship that caused the Deduplicate"
        }
        RewriteRule::InnerJoins => {
            "passing orphan_keys evidence, no NULL value possible in from_columns, and every row \
             of both sides reaches the join"
        }
        RewriteRule::MergeScans => "the branches are identical except for their components",
        RewriteRule::UseSummary => {
            "equivalence, grain, policy preservation and freshness hold, and the summary has the \
             smallest row_count_estimate"
        }
        RewriteRule::FoldConstants => "the home dataset's row-policy filter is the constant FALSE",
    }
}

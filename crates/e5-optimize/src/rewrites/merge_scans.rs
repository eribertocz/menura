//! E5-3.4, E5-3.4a: evaluates the components of identical branches in one `Aggregate`.

use menura_representations::logical_plan::{
    AggregateComponent, NamedExpression, Operation, OutputSchema, PlanNode, StitchColumn,
};
use menura_representations::names::ColumnName;

use crate::context::RewriteContext;
use crate::record::{NodePath, RewriteOutcome, RewriteRecord, RewriteRule, RewriteSubject};

/// The failed condition of a branch that holds a semi-additive plan (E5-3.4a).
const SEMI_ADDITIVE: &str = "the branch has a Population, Snapshot or Periods node (E5-3.4a)";
/// The failed condition of a branch no earlier branch matches (E5-3.4a).
const NOT_IDENTICAL: &str = "no earlier branch is identical except for its components (E5-3.4a)";

/// Applies the rewrite to `plan` and adds a record of every case it considered to `records`:
/// for each branch of the `Stitch` after the first, whether it was merged into an earlier one.
/// The `Stitch` is found below the `Project`, `Sort` and `Limit` nodes E4 puts above it.
pub(super) fn rewrite(
    plan: PlanNode,
    _context: &RewriteContext<'_>,
    records: &mut Vec<RewriteRecord>,
) -> PlanNode {
    rewrite_at(plan, Vec::new(), records)
}

/// Rewrites the plan rooted at `plan`, the node at `path` of the input plan.
fn rewrite_at(plan: PlanNode, mut path: Vec<usize>, records: &mut Vec<RewriteRecord>) -> PlanNode {
    let schema = plan.schema().clone();
    let origin = plan.origin().clone();
    let operation = match plan.operation().clone() {
        Operation::Stitch {
            branches,
            keys,
            columns,
        } => return merge_stitch(plan, branches, keys, columns, &path, records),
        Operation::Project { input, expressions } => {
            path.push(0);
            Operation::Project {
                input: Box::new(rewrite_at(*input, path, records)),
                expressions,
            }
        }
        Operation::Sort { input, items } => {
            path.push(0);
            Operation::Sort {
                input: Box::new(rewrite_at(*input, path, records)),
                items,
            }
        }
        Operation::Limit { input, rows } => {
            path.push(0);
            Operation::Limit {
                input: Box::new(rewrite_at(*input, path, records)),
                rows,
            }
        }
        _ => return plan,
    };
    PlanNode::new(operation, schema, origin)
}

/// The `Stitch` node `stitch`, at `path`, with each branch merged into the first earlier
/// branch identical to it (E5-3.4a); the merged branch alone when one branch remains.
fn merge_stitch(
    stitch: PlanNode,
    branches: Vec<PlanNode>,
    keys: Vec<ColumnName>,
    columns: Vec<StitchColumn>,
    path: &[usize],
    records: &mut Vec<RewriteRecord>,
) -> PlanNode {
    let targets = merge_targets(&branches, path, records);
    let mut merged: Vec<PlanNode> = Vec::new();
    for (branch, target) in branches.into_iter().zip(&targets) {
        match merged.get_mut(*target) {
            Some(first) => *first = with_components_of(first, &branch),
            None => merged.push(branch),
        }
    }
    if merged.len() == targets.len() {
        return stitch;
    }
    if merged.len() == 1
        && let Some(only) = merged.pop()
    {
        return only;
    }
    let columns = columns
        .into_iter()
        .map(|column| StitchColumn {
            branch: targets.get(column.branch).copied().unwrap_or(column.branch),
            ..column
        })
        .collect();
    PlanNode::new(
        Operation::Stitch {
            branches: merged,
            keys,
            columns,
        },
        stitch.schema().clone(),
        stitch.origin().clone(),
    )
}

/// For each branch, the position in the merged `Stitch` of the branch it ends up in, and a
/// record for each branch after the first.
fn merge_targets(
    branches: &[PlanNode],
    path: &[usize],
    records: &mut Vec<RewriteRecord>,
) -> Vec<usize> {
    let mut targets: Vec<usize> = Vec::with_capacity(branches.len());
    let mut kept: Vec<&PlanNode> = Vec::new();
    for (position, branch) in branches.iter().enumerate() {
        let mergeable = !has_semi_additive_node(branch);
        let earlier = mergeable
            .then(|| {
                kept.iter()
                    .position(|first| identical_except_components(first, branch))
            })
            .flatten();
        if position > 0 {
            let outcome = match (mergeable, earlier) {
                (false, _) => rejected(SEMI_ADDITIVE),
                (true, None) => rejected(NOT_IDENTICAL),
                (true, Some(_)) => RewriteOutcome::Applied,
            };
            let mut steps = path.to_vec();
            steps.push(position);
            records.push(RewriteRecord {
                rule: RewriteRule::MergeScans,
                subject: RewriteSubject::Node(NodePath::new(steps)),
                outcome,
            });
        }
        targets.push(earlier.unwrap_or(kept.len()));
        if earlier.is_none() {
            kept.push(branch);
        }
    }
    targets
}

fn rejected(failed_condition: &str) -> RewriteOutcome {
    RewriteOutcome::Rejected {
        failed_condition: failed_condition.to_owned(),
    }
}

/// Whether the plan rooted at `node` has a `Population`, `Snapshot` or `Periods` node.
fn has_semi_additive_node(node: &PlanNode) -> bool {
    matches!(
        node.operation(),
        Operation::Population { .. } | Operation::Snapshot { .. } | Operation::Periods { .. }
    ) || node.inputs().into_iter().any(has_semi_additive_node)
}

/// The input and keys of `branch`, when it ends with its `Aggregate`.
fn aggregate_parts(branch: &PlanNode) -> Option<(&PlanNode, &[NamedExpression])> {
    match branch.operation() {
        Operation::Aggregate { input, keys, .. } => Some((input, keys)),
        _ => None,
    }
}

/// Whether `first` and `other` are identical except for their components (E5-3.4a): both end
/// with an `Aggregate` with the same keys over equal plans, so the same home `Scan`, row-policy
/// nodes, `Join` nodes in order, query filter and `Deduplicate`.
fn identical_except_components(first: &PlanNode, other: &PlanNode) -> bool {
    match (aggregate_parts(first), aggregate_parts(other)) {
        (Some(first), Some(other)) => first == other,
        _ => false,
    }
}

/// The `Aggregate` `first` with the components of the identical `other` added after its own,
/// in their order and with their names, and their output columns after its own.
fn with_components_of(first: &PlanNode, other: &PlanNode) -> PlanNode {
    let (
        Operation::Aggregate {
            input,
            keys,
            components,
        },
        Operation::Aggregate {
            components: added, ..
        },
    ) = (first.operation(), other.operation())
    else {
        return first.clone();
    };
    let components: Vec<AggregateComponent> = components.iter().chain(added).cloned().collect();
    let mut columns = first.schema().columns().to_vec();
    columns.extend(
        other
            .schema()
            .columns()
            .iter()
            .filter(|column| added.iter().any(|component| component.name == column.name))
            .cloned(),
    );
    PlanNode::new(
        Operation::Aggregate {
            input: input.clone(),
            keys: keys.clone(),
            components,
        },
        OutputSchema::new(columns),
        first.origin().clone(),
    )
}

#[cfg(test)]
#[path = "merge_scans_tests.rs"]
mod tests;

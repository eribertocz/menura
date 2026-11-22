//! E5-3.4, E5-3.4a: evaluates the components of identical branches in one `Aggregate`.

use menura_model::internal_model::InternalModel;
use menura_representations::logical_plan::{
    AggregateComponent, NamedExpression, Operation, OutputSchema, PlanNode, StitchColumn,
};
use menura_representations::names::ColumnName;

use crate::context::RewriteContext;
use crate::plan_inputs::{home_dataset, map_inputs};
use crate::record::{NodePath, RewriteOutcome, RewriteRecord, RewriteRule, RewriteSubject};

#[path = "merge_scans_rows.rs"]
mod rows;

/// The failed condition of a branch that holds a semi-additive plan (E5-3.4a).
const SEMI_ADDITIVE: &str = "the branch has a Population, Snapshot or Periods node (E5-3.4a)";
/// The failed condition of a branch no earlier branch matches (E5-3.4a).
const NOT_IDENTICAL: &str = "no earlier branch is identical except for its components (E5-3.4a)";

/// Applies the rewrite to `plan` and adds a record of every case it considered to `records`:
/// for each branch of the `Stitch` after the first, whether it was merged into an earlier one.
/// The `Stitch` is found below the `Project`, `Sort` and `Limit` nodes E4 puts above it.
pub(super) fn rewrite(
    plan: PlanNode,
    context: &RewriteContext<'_>,
    records: &mut Vec<RewriteRecord>,
) -> PlanNode {
    rewrite_at(plan, Vec::new(), context.model, records)
}

/// Rewrites the plan rooted at `plan`, the node at `path` of the input plan.
fn rewrite_at(
    plan: PlanNode,
    mut path: Vec<usize>,
    model: &InternalModel,
    records: &mut Vec<RewriteRecord>,
) -> PlanNode {
    let schema = plan.schema().clone();
    let origin = plan.origin().clone();
    let operation = match plan.operation().clone() {
        Operation::Stitch {
            branches,
            keys,
            columns,
        } => return merge_stitch(plan, branches, keys, columns, &path, model, records),
        Operation::Project { input, expressions } => {
            path.push(0);
            Operation::Project {
                input: Box::new(rewrite_at(*input, path, model, records)),
                expressions,
            }
        }
        Operation::Sort { input, items } => {
            path.push(0);
            Operation::Sort {
                input: Box::new(rewrite_at(*input, path, model, records)),
                items,
            }
        }
        Operation::Limit { input, rows } => {
            path.push(0);
            Operation::Limit {
                input: Box::new(rewrite_at(*input, path, model, records)),
                rows,
            }
        }
        Operation::Lookup { .. } => {
            return map_inputs(&plan, |position, input| {
                rewrite_at(input.clone(), child(&path, position), model, records)
            });
        }
        _ => return plan,
    };
    PlanNode::new(operation, schema, origin)
}

/// `path` followed by `position`: the path of an input of the node at `path`.
fn child(path: &[usize], position: usize) -> Vec<usize> {
    let mut child = path.to_vec();
    child.push(position);
    child
}

/// The `Stitch` node `stitch`, at `path`, with each branch merged into the first earlier
/// branch identical to it (E5-3.4a); the merged branch alone when one branch remains.
fn merge_stitch(
    stitch: PlanNode,
    branches: Vec<PlanNode>,
    keys: Vec<ColumnName>,
    columns: Vec<StitchColumn>,
    path: &[usize],
    model: &InternalModel,
    records: &mut Vec<RewriteRecord>,
) -> PlanNode {
    let targets = merge_targets(&branches, path, model, records);
    let mut merged: Vec<PlanNode> = Vec::new();
    for (branch, target) in branches.into_iter().zip(&targets) {
        match merged.get_mut(*target) {
            Some(first) => *first = with_components_of(first, &branch, model),
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
/// record for each branch whose home dataset an earlier branch also reads: the rewrite is
/// considered only for branches with the same home `Scan` (E5-3.8).
fn merge_targets(
    branches: &[PlanNode],
    path: &[usize],
    model: &InternalModel,
    records: &mut Vec<RewriteRecord>,
) -> Vec<usize> {
    let mut targets: Vec<usize> = Vec::with_capacity(branches.len());
    let mut kept: Vec<&PlanNode> = Vec::new();
    for (position, branch) in branches.iter().enumerate() {
        let mergeable = !has_semi_additive_node(branch);
        let earlier = mergeable
            .then(|| {
                kept.iter()
                    .position(|first| identical_except_components(first, branch, model))
            })
            .flatten();
        let home = home_dataset(branch);
        let shares_home = branches[..position]
            .iter()
            .any(|earlier_branch| home.is_some() && home_dataset(earlier_branch) == home);
        if shares_home {
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
/// with an `Aggregate` with the same keys over plans that read the same rows: the same home
/// `Scan`, whatever columns it projects, row-policy nodes, `Join` nodes in order, query filter
/// and `Deduplicate`.
fn identical_except_components(first: &PlanNode, other: &PlanNode, model: &InternalModel) -> bool {
    match (aggregate_parts(first), aggregate_parts(other)) {
        (Some((input, keys)), Some((other_input, other_keys))) => {
            keys == other_keys && rows::same_rows(input, other_input, model)
        }
        _ => false,
    }
}

/// The `Aggregate` `first` with the components of the identical `other` added after its own,
/// in their order and with their names, and their output columns after its own. Its home `Scan`
/// also projects the columns of the home `Scan` of `other`.
fn with_components_of(first: &PlanNode, other: &PlanNode, model: &InternalModel) -> PlanNode {
    let (
        Operation::Aggregate {
            input,
            keys,
            components,
        },
        Operation::Aggregate {
            input: other_input,
            components: added,
            ..
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
            input: Box::new(rows::with_columns_of(input, other_input, model)),
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

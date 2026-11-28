//! E5-3.2, E5-3.2a: removes a `Deduplicate` node with `key_uniqueness` evidence, or `population`
//! evidence for a temporal relationship (VAL-2.14), for every join of its branch.

use menura_model::internal_model::{CheckKind, InternalModel, NormalizedName};
use menura_representations::logical_plan::{Operation, OutputSchema, PlanNode};
use menura_representations::names::ColumnName;

use crate::context::RewriteContext;
use crate::plan_inputs::input_mut;
use crate::record::{NodePath, RewriteOutcome, RewriteRecord, RewriteRule, RewriteSubject};

/// The failed condition of a `Deduplicate` whose input is not a branch of E4-4.0: a home
/// `Scan` under `Filter`, `SemiJoin` and `Join` nodes only.
const NOT_A_BRANCH: &str =
    "the input of the Deduplicate is not a home Scan under Filter, SemiJoin and Join nodes";

/// Removes every `Deduplicate` of `plan` whose branch has `key_uniqueness` evidence for each of
/// its joins (E5-3.2a), and the synthesized row identity of its home `Scan` with it (E4-4.0f).
/// Adds one record per `Deduplicate` to `records`: applied, or rejected with the first join
/// that has no evidence (E5-3.8).
pub(super) fn rewrite(
    plan: PlanNode,
    context: &RewriteContext<'_>,
    records: &mut Vec<RewriteRecord>,
) -> PlanNode {
    rewrite_node(&plan, &mut Vec::new(), context, records).unwrap_or(plan)
}

/// The rewritten `node`, at `path` in the plan, or `None` when nothing under it changes.
fn rewrite_node(
    node: &PlanNode,
    path: &mut Vec<usize>,
    context: &RewriteContext<'_>,
    records: &mut Vec<RewriteRecord>,
) -> Option<PlanNode> {
    if let Operation::Deduplicate {
        input, identity, ..
    } = node.operation()
    {
        return consider(input, identity, path, context, records);
    }
    let mut replaced = Vec::new();
    for (index, input) in node.inputs().into_iter().enumerate() {
        path.push(index);
        if let Some(rewritten) = rewrite_node(input, path, context, records) {
            replaced.push((index, rewritten));
        }
        path.pop();
    }
    if replaced.is_empty() {
        return None;
    }
    let mut operation = node.operation().clone();
    for (index, rewritten) in replaced {
        if let Some(slot) = input_mut(&mut operation, index) {
            *slot = rewritten;
        }
    }
    Some(PlanNode::new(
        operation,
        node.schema().clone(),
        node.origin().clone(),
    ))
}

/// Records whether the `Deduplicate` at `path`, over `input` with the row identity columns
/// `identity`, is removed, and returns `input` without those columns when it is.
fn consider(
    input: &PlanNode,
    identity: &[ColumnName],
    path: &[usize],
    context: &RewriteContext<'_>,
    records: &mut Vec<RewriteRecord>,
) -> Option<PlanNode> {
    let failed_condition = failed_condition(input, context);
    let removed = failed_condition
        .is_none()
        .then(|| without_identity(input, identity));
    let outcome = match failed_condition {
        None => RewriteOutcome::Applied,
        Some(failed_condition) => RewriteOutcome::Rejected { failed_condition },
    };
    records.push(RewriteRecord {
        rule: RewriteRule::SkipDeduplication,
        subject: RewriteSubject::Node(NodePath::new(path.to_vec())),
        outcome,
    });
    removed
}

/// One `Join` of a branch: its relationship and the dataset its right input scans.
struct BranchJoin<'plan> {
    relationship: &'plan NormalizedName,
    joined: &'plan NormalizedName,
}

/// The first condition of E5-3.2a that the branch `input` fails, in path order of its joins,
/// or `None` when every join follows its relationship and has `key_uniqueness` evidence.
fn failed_condition(input: &PlanNode, context: &RewriteContext<'_>) -> Option<String> {
    let Some(joins) = branch_joins(input) else {
        return Some(NOT_A_BRANCH.to_owned());
    };
    joins.iter().find_map(|join| {
        let name = join.relationship.as_str();
        if !follows_relationship(join, context.model) {
            Some(format!(
                "the Join of relationship {name} does not follow it from `from` to `to` (E5-3.2a)"
            ))
        } else {
            missing_uniqueness(join.relationship, context)
        }
    })
}

/// The failed condition of a `Join` along `relationship` that has no uniqueness evidence:
/// `key_uniqueness` evidence for the relationship, or, for a temporal relationship, `population`
/// evidence for its population (E5-3.2a, VAL-2.14); `None` when it has it.
fn missing_uniqueness(
    relationship: &NormalizedName,
    context: &RewriteContext<'_>,
) -> Option<String> {
    let name = relationship.as_str();
    let range = context
        .model
        .relationship(relationship)
        .and_then(|found| context.model.temporal_range(found));
    match range {
        Some(range)
            if !context
                .evidence
                .holds(CheckKind::Population, range.population) =>
        {
            Some(format!(
                "no passing population evidence for population {} of temporal relationship \
                 {name} (E5-3.2a, VAL-2.14)",
                range.population.as_str()
            ))
        }
        Some(_) => None,
        None if !context
            .evidence
            .holds(CheckKind::KeyUniqueness, relationship) =>
        {
            Some(format!(
                "no passing key_uniqueness evidence for relationship {name} (E5-3.2a)"
            ))
        }
        None => None,
    }
}

/// The joins of the branch `input`, in path order (E4-4.0c), or `None` when `input` is not a
/// home `Scan` under `Filter`, `SemiJoin` and `Join` nodes.
fn branch_joins(input: &PlanNode) -> Option<Vec<BranchJoin<'_>>> {
    let mut joins = Vec::new();
    let mut node = input;
    loop {
        match node.operation() {
            Operation::Join {
                left,
                right,
                relationship,
                ..
            } => {
                joins.push(BranchJoin {
                    relationship,
                    joined: scanned_dataset(right)?,
                });
                node = left;
            }
            Operation::Filter { input, .. } | Operation::SemiJoin { input, .. } => node = input,
            Operation::Scan { .. } => {
                joins.reverse();
                return Some(joins);
            }
            _ => return None,
        }
    }
}

/// The dataset of the `Scan` under the `Filter` and `SemiJoin` nodes of the joined input
/// `node` (E4-6.5c), or `None` when `node` is not of that form.
fn scanned_dataset(node: &PlanNode) -> Option<&NormalizedName> {
    match node.operation() {
        Operation::Scan { dataset, .. } => Some(dataset),
        Operation::Filter { input, .. } | Operation::SemiJoin { input, .. } => {
            scanned_dataset(input)
        }
        _ => None,
    }
}

/// Whether `join` follows its relationship from `from` to `to`: its right input scans the
/// relationship's `to` dataset. The left input is the branch built from `from` (E4-4.0c).
fn follows_relationship(join: &BranchJoin<'_>, model: &InternalModel) -> bool {
    model
        .relationship(join.relationship)
        .is_some_and(|relationship| &relationship.to == join.joined)
}

/// The branch `node` without the row identity columns `identity`: removed from the output of
/// every node from the home `Scan` up, and from the home `Scan`'s `row_identity`.
fn without_identity(node: &PlanNode, identity: &[ColumnName]) -> PlanNode {
    let columns = node
        .schema()
        .columns()
        .iter()
        .filter(|column| !identity.contains(&column.name))
        .cloned()
        .collect();
    let operation = match node.operation() {
        Operation::Scan { dataset, .. } => Operation::Scan {
            dataset: dataset.clone(),
            row_identity: None,
        },
        Operation::Filter { input, predicate } => Operation::Filter {
            input: Box::new(without_identity(input, identity)),
            predicate: predicate.clone(),
        },
        Operation::SemiJoin {
            input,
            relationship,
            on,
            temporal,
            related_all,
            related_visible,
        } => Operation::SemiJoin {
            input: Box::new(without_identity(input, identity)),
            relationship: relationship.clone(),
            on: on.clone(),
            temporal: temporal.clone(),
            related_all: related_all.clone(),
            related_visible: related_visible.clone(),
        },
        Operation::Join {
            left,
            right,
            relationship,
            kind,
            on,
            temporal,
            right_prefix,
        } => Operation::Join {
            left: Box::new(without_identity(left, identity)),
            right: right.clone(),
            relationship: relationship.clone(),
            kind: *kind,
            on: on.clone(),
            temporal: temporal.clone(),
            right_prefix: right_prefix.clone(),
        },
        other => other.clone(),
    };
    PlanNode::new(operation, OutputSchema::new(columns), node.origin().clone())
}

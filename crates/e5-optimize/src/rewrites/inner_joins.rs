//! E5-3.3, E5-3.3a: changes a `left` join to `inner` with `orphan_keys` evidence, or
//! `temporal_orphans` evidence for a temporal relationship (VAL-2.15), and the conditions of
//! E5-3.3a.
//!
//! A chain of `Join` nodes is examined in path order, from the `Join` over the home side to
//! the last one, because condition (2) of E5-3.3a depends on the joins before it.

use std::collections::BTreeSet;

use menura_model::expression::{Expression, Reference};
use menura_model::internal_model::{CheckKind, NormalizedName, Relationship};
use menura_representations::logical_plan::{
    JoinCondition, JoinKind, Operation, PlanNode, TemporalCoverage,
};
use menura_representations::names::{ColumnName, ColumnPrefix};

use crate::context::RewriteContext;
use crate::plan_inputs::map_inputs;
use crate::record::{NodePath, RewriteOutcome, RewriteRecord, RewriteRule, RewriteSubject};

/// Applies the rewrite to `plan` and adds a record of every `left` join it considered to
/// `records`, in path order.
pub(super) fn rewrite(
    plan: PlanNode,
    context: &RewriteContext<'_>,
    records: &mut Vec<RewriteRecord>,
) -> PlanNode {
    if !has_join(&plan) {
        return plan;
    }
    rewrite_node(&plan, Vec::new(), context, records)
}

/// Whether the plan rooted at `node` has a `Join` node.
fn has_join(node: &PlanNode) -> bool {
    matches!(node.operation(), Operation::Join { .. }) || node.inputs().into_iter().any(has_join)
}

/// `node`, at `path` in the input plan, with the rewrite applied to every join chain in it.
fn rewrite_node(
    node: &PlanNode,
    path: Vec<usize>,
    context: &RewriteContext<'_>,
    records: &mut Vec<RewriteRecord>,
) -> PlanNode {
    if matches!(node.operation(), Operation::Join { .. }) {
        return rewrite_chain(node, path, context, records);
    }
    map_inputs(node, |index, input| {
        rewrite_node(input, child(&path, index), context, records)
    })
}

/// The path of input `index` of the node at `path`.
fn child(path: &[usize], index: usize) -> Vec<usize> {
    let mut steps = path.to_vec();
    steps.push(index);
    steps
}

/// One `Join` of a chain, with its path in the input plan.
struct ChainJoin<'plan> {
    /// The `Join` node.
    node: &'plan PlanNode,
    /// Its path in the input plan.
    path: Vec<usize>,
    /// Its joined side.
    right: &'plan PlanNode,
    /// The relationship it joins along.
    relationship: &'plan NormalizedName,
    /// Its kind in the input plan.
    kind: JoinKind,
    /// Its key equalities.
    on: &'plan [JoinCondition],
    /// Its coverage condition, along a temporal relationship (VAL-2.8).
    temporal: &'plan Option<TemporalCoverage>,
    /// The prefix of the columns of its joined side.
    right_prefix: &'plan ColumnPrefix,
}

/// The `Join` at `path` with the joins below it through `left`, in path order, and the home
/// side below the first one with its path.
fn unwind(top: &PlanNode, path: Vec<usize>) -> (Vec<ChainJoin<'_>>, &PlanNode, Vec<usize>) {
    let mut joins = Vec::new();
    let mut node = top;
    let mut node_path = path;
    while let Operation::Join {
        left,
        right,
        relationship,
        kind,
        on,
        temporal,
        right_prefix,
    } = node.operation()
    {
        let left_path = child(&node_path, 0);
        joins.push(ChainJoin {
            node,
            path: node_path,
            right,
            relationship,
            kind: *kind,
            on,
            temporal,
            right_prefix,
        });
        node = left;
        node_path = left_path;
    }
    joins.reverse();
    (joins, node, node_path)
}

/// The chain whose last `Join` is `top`, at `path`, with each `left` join changed to `inner`
/// when E5-3.3 and E5-3.3a hold, in path order.
fn rewrite_chain(
    top: &PlanNode,
    path: Vec<usize>,
    context: &RewriteContext<'_>,
    records: &mut Vec<RewriteRecord>,
) -> PlanNode {
    let (joins, home, home_path) = unwind(top, path);
    let mut sides = vec![Side {
        columns: columns(home),
        reads_every_row: true,
    }];
    let mut kinds = Vec::with_capacity(joins.len());
    for join in &joins {
        let decided = match join.kind {
            JoinKind::Inner => JoinKind::Inner,
            JoinKind::Left => match decide(join, &sides, context) {
                Some(outcome) => {
                    let decided = outcome.map_or(JoinKind::Left, |()| JoinKind::Inner);
                    records.push(record(&join.path, outcome));
                    decided
                }
                None => JoinKind::Left,
            },
        };
        sides.push(Side {
            columns: columns(join.right),
            reads_every_row: decided == JoinKind::Inner,
        });
        kinds.push(decided);
    }
    let mut node = rewrite_node(home, home_path, context, records);
    for (join, kind) in joins.iter().zip(kinds) {
        node = rebuilt_join(join, node, kind, context, records);
    }
    node
}

/// One side of a chain that a later `Join` can join from: the home side, or the joined side of
/// a `Join`.
struct Side<'plan> {
    /// The columns it contributes.
    columns: BTreeSet<&'plan str>,
    /// Whether every one of its rows reaches the joins after it: true for the home side and for
    /// a side joined by an `inner` join (E5-3.3a (2)).
    reads_every_row: bool,
}

/// The names of the columns `node` produces.
fn columns(node: &PlanNode) -> BTreeSet<&str> {
    node.schema()
        .columns()
        .iter()
        .map(|column| column.name.as_str())
        .collect()
}

/// `join` with `left` as its left input, its joined side rewritten, and `kind`.
fn rebuilt_join(
    join: &ChainJoin<'_>,
    left: PlanNode,
    kind: JoinKind,
    context: &RewriteContext<'_>,
    records: &mut Vec<RewriteRecord>,
) -> PlanNode {
    let right = rewrite_node(join.right, child(&join.path, 1), context, records);
    PlanNode::new(
        Operation::Join {
            left: Box::new(left),
            right: Box::new(right),
            relationship: join.relationship.clone(),
            kind,
            on: join.on.to_vec(),
            temporal: join.temporal.clone(),
            right_prefix: join.right_prefix.clone(),
        },
        join.node.schema().clone(),
        join.node.origin().clone(),
    )
}

/// The record of the `left` join at `path`: applied, or rejected with its failed condition.
fn record(path: &[usize], outcome: Result<(), FailedCondition>) -> RewriteRecord {
    RewriteRecord {
        rule: RewriteRule::InnerJoins,
        subject: RewriteSubject::Node(NodePath::new(path.to_vec())),
        outcome: match outcome {
            Ok(()) => RewriteOutcome::Applied,
            Err(failed) => RewriteOutcome::Rejected {
                failed_condition: failed.text().to_owned(),
            },
        },
    }
}

/// The condition that rejects a considered `left` join (E5-3.8): the evidence of E5-3.3. The
/// conditions of E5-3.3a are its structural precondition: a join that fails one of them is not
/// considered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FailedCondition {
    /// E5-3.3: no passing `orphan_keys` evidence.
    Evidence,
    /// E5-3.3, VAL-2.15: no passing `temporal_orphans` evidence for a temporal relationship.
    TemporalEvidence,
}

impl FailedCondition {
    /// The condition, as the rule states it.
    fn text(self) -> &'static str {
        match self {
            Self::Evidence => "passing `orphan_keys` evidence for the relationship (CAT-7.3)",
            Self::TemporalEvidence => {
                "passing `temporal_orphans` evidence for the temporal relationship (CAT-7.3, \
                 VAL-2.15)"
            }
        }
    }
}

/// Whether the `left` join `join`, after the `sides` of the chain before it, may become
/// `inner`: `None` when a condition of E5-3.3a fails, so the rewrite is not considered
/// (E5-3.8); otherwise applied, or rejected without `orphan_keys` evidence.
fn decide(
    join: &ChainJoin<'_>,
    sides: &[Side<'_>],
    context: &RewriteContext<'_>,
) -> Option<Result<(), FailedCondition>> {
    let relationship = context.model.relationship(join.relationship)?;
    let from_time = join.temporal.as_ref().map(|coverage| &coverage.from_time);
    let structural = from_columns_are_not_null(relationship, context)
        && from_side_reads_every_row(join.on, from_time, sides)
        && is_bare_scan_of(join.right, &relationship.to);
    if !structural {
        return None;
    }
    let (kind, failed) = if relationship.temporal.is_some() {
        (
            CheckKind::TemporalOrphans,
            FailedCondition::TemporalEvidence,
        )
    } else {
        (CheckKind::OrphanKeys, FailedCondition::Evidence)
    };
    if context.evidence.holds(kind, join.relationship) {
        Some(Ok(()))
    } else {
        Some(Err(failed))
    }
}

/// Whether every `from_columns` field of `relationship`, and its `from_time` field when it is
/// temporal (VAL-2.15), is a catalog column of its `from` dataset, read as is, that the catalog
/// schema declares `nullable: false` (E5-3.3a (1)). E0 takes the nullability of a column
/// reference from the admitted catalog schema (E0H-2.2).
fn from_columns_are_not_null(relationship: &Relationship, context: &RewriteContext<'_>) -> bool {
    let Some(from) = context.model.dataset(&relationship.from) else {
        return false;
    };
    let from_time = relationship
        .temporal
        .as_ref()
        .map(|temporal| &temporal.from_time);
    relationship
        .from_columns
        .iter()
        .chain(from_time)
        .all(|name| {
            from.field(name).is_some_and(|field| {
                matches!(
                    field.expression,
                    Some(Expression::Reference(Reference {
                        qualifier: None,
                        ..
                    }))
                ) && field.typed.as_ref().is_some_and(|typed| !typed.nullable)
            })
        })
}

/// Whether the columns of the left side of `on`, and `from_time` along a temporal
/// relationship, all come from one side of the chain, and that side is the home side or a side
/// joined by an `inner` join (E5-3.3a (2)).
fn from_side_reads_every_row(
    on: &[JoinCondition],
    from_time: Option<&ColumnName>,
    sides: &[Side<'_>],
) -> bool {
    let Some(first) = on.first() else {
        return false;
    };
    sides
        .iter()
        .find(|side| side.columns.contains(first.left.as_str()))
        .is_some_and(|side| {
            side.reads_every_row
                && on
                    .iter()
                    .map(|condition| &condition.left)
                    .chain(from_time)
                    .all(|column| side.columns.contains(column.as_str()))
        })
}

/// Whether `node` is a `Scan` of `dataset` with no `Filter` or `SemiJoin` over it: no
/// applicable policy and no propagation filters it for this user, and `G` is not `FALSE`
/// (E5-3.3a (3), E4-6.5c).
fn is_bare_scan_of(node: &PlanNode, dataset: &NormalizedName) -> bool {
    matches!(node.operation(), Operation::Scan { dataset: scanned, .. } if scanned == dataset)
}

#[cfg(test)]
#[path = "inner_joins_side_tests.rs"]
mod side_tests;

//! E5-3.6: turns a branch whose home row-policy filter is the constant `FALSE` into an empty input.
//!
//! The folded branch keeps its `Aggregate`, its `Project` of expression dimensions, its query
//! filter and its `Join` nodes with their joined rows unchanged, so every column the
//! `Aggregate` reads keeps its name and type. The `Deduplicate` over the joins is dropped, and
//! below the first `Join` only the home `Scan` and its row-policy `Filter` remain: the
//! propagation `SemiJoin` nodes over that `Filter` are dropped too. Neither can change an input
//! that has no row. The
//! `Aggregate` then yields no row when it has keys and one row of empty-input values when it
//! has none (E4-5.0b), as the base plan does over no rows. The `FALSE` filter itself stays
//! (E5-3.7). A joined dataset whose filter is `FALSE` is not folded.

use menura_model::expression::{BinaryOperator, Expression, Literal};
use menura_representations::logical_plan::{Operation, PlanNode};
use menura_representations::origin::Origin;

use crate::context::RewriteContext;
use crate::record::{NodePath, RewriteOutcome, RewriteRecord, RewriteRule, RewriteSubject};

/// Applies the rewrite to `plan` and adds a record of every branch it folded to `records`.
pub(super) fn rewrite(
    plan: PlanNode,
    _context: &RewriteContext<'_>,
    records: &mut Vec<RewriteRecord>,
) -> PlanNode {
    fold_branches(&plan, &mut Vec::new(), records).unwrap_or(plan)
}

/// `node` with every branch `Aggregate` below its `Limit`, `Sort`, `Project` and `Stitch`
/// nodes folded when its home row-policy filter is `FALSE`; `None` when no branch is folded.
/// `steps` is the path of `node`.
fn fold_branches(
    node: &PlanNode,
    steps: &mut Vec<usize>,
    records: &mut Vec<RewriteRecord>,
) -> Option<PlanNode> {
    match node.operation() {
        Operation::Aggregate { input, .. } => {
            let mut path = steps.clone();
            path.push(0);
            let folded = fold_rows(input, &mut path)?;
            records.push(applied(path));
            Some(with_input(node, folded))
        }
        Operation::Limit { input, .. }
        | Operation::Sort { input, .. }
        | Operation::Project { input, .. } => {
            steps.push(0);
            let folded = fold_branches(input, steps, records);
            steps.pop();
            folded.map(|folded| with_input(node, folded))
        }
        Operation::Stitch { branches, .. } => fold_stitch(node, branches, steps, records),
        _ => None,
    }
}

/// The `Stitch` `node` with each of its `branches` folded; `None` when none is folded.
fn fold_stitch(
    node: &PlanNode,
    branches: &[PlanNode],
    steps: &mut Vec<usize>,
    records: &mut Vec<RewriteRecord>,
) -> Option<PlanNode> {
    let mut folded = Vec::with_capacity(branches.len());
    for (index, branch) in branches.iter().enumerate() {
        steps.push(index);
        folded.push(fold_branches(branch, steps, records));
        steps.pop();
    }
    if folded.iter().all(Option::is_none) {
        return None;
    }
    let Operation::Stitch { keys, columns, .. } = node.operation() else {
        return None;
    };
    let branches = folded
        .into_iter()
        .zip(branches)
        .map(|(folded, branch)| folded.unwrap_or_else(|| branch.clone()))
        .collect();
    let operation = Operation::Stitch {
        branches,
        keys: keys.clone(),
        columns: columns.clone(),
    };
    Some(PlanNode::new(
        operation,
        node.schema().clone(),
        node.origin().clone(),
    ))
}

/// The folded rows of a branch, whose plan below its `Aggregate` is `node`, when its home
/// row-policy `Filter` is `FALSE`; `None` otherwise. On success `steps` ends at that `Filter`.
///
/// The rows are followed from the `Aggregate` to the home `Scan` through the nodes E4-4 puts
/// between them: `Deduplicate`, `Project`, the query-filter `Filter`, the `left` input of each
/// `Join` and the propagation `SemiJoin` nodes. A branch with any other node there, such as a
/// `Snapshot`, is not folded.
fn fold_rows(node: &PlanNode, steps: &mut Vec<usize>) -> Option<PlanNode> {
    match node.operation() {
        Operation::Filter { input, predicate } if is_scan(input) => {
            let row_policy = matches!(node.origin(), Origin::Model(_));
            (row_policy && is_constant_false(predicate)).then(|| node.clone())
        }
        Operation::Deduplicate { input, .. } | Operation::SemiJoin { input, .. } => {
            steps.push(0);
            fold_rows(input, steps)
        }
        Operation::Filter { input, .. } | Operation::Project { input, .. } => {
            steps.push(0);
            fold_rows(input, steps).map(|folded| with_input(node, folded))
        }
        Operation::Join { left, .. } => {
            steps.push(0);
            fold_rows(left, steps).map(|folded| with_input(node, folded))
        }
        _ => None,
    }
}

/// Whether `node` is a `Scan`.
fn is_scan(node: &PlanNode) -> bool {
    matches!(node.operation(), Operation::Scan { .. })
}

/// Whether `predicate` is the constant `FALSE`, alone or as an operand of `AND`: the grant `G`
/// of E4-6.4 in the row filter `G AND R` of E4-6.2.
fn is_constant_false(predicate: &Expression) -> bool {
    match predicate {
        Expression::Literal(Literal::Boolean(value)) => !value,
        Expression::Binary {
            operator: BinaryOperator::And,
            left,
            right,
        } => is_constant_false(left) || is_constant_false(right),
        _ => false,
    }
}

/// `node` with its first input replaced by `input`: the `input` of a `Filter`, `Aggregate`,
/// `Project`, `Sort` or `Limit`, and the `left` input of a `Join`. Any other node is returned
/// unchanged, which `fold_branches` and `fold_rows` never ask for.
fn with_input(node: &PlanNode, input: PlanNode) -> PlanNode {
    let input = Box::new(input);
    let operation = match node.operation() {
        Operation::Filter { predicate, .. } => Operation::Filter {
            input,
            predicate: predicate.clone(),
        },
        Operation::Aggregate {
            keys, components, ..
        } => Operation::Aggregate {
            input,
            keys: keys.clone(),
            components: components.clone(),
        },
        Operation::Project { expressions, .. } => Operation::Project {
            input,
            expressions: expressions.clone(),
        },
        Operation::Sort { items, .. } => Operation::Sort {
            input,
            items: items.clone(),
        },
        Operation::Limit { rows, .. } => Operation::Limit { input, rows: *rows },
        Operation::Join {
            right,
            relationship,
            kind,
            on,
            right_prefix,
            ..
        } => Operation::Join {
            left: input,
            right: right.clone(),
            relationship: relationship.clone(),
            kind: *kind,
            on: on.clone(),
            right_prefix: right_prefix.clone(),
        },
        _ => return node.clone(),
    };
    PlanNode::new(operation, node.schema().clone(), node.origin().clone())
}

/// The record of a branch folded at the home row-policy `Filter` at `steps`.
fn applied(steps: Vec<usize>) -> RewriteRecord {
    RewriteRecord {
        rule: RewriteRule::FoldConstants,
        subject: RewriteSubject::Node(NodePath::new(steps)),
        outcome: RewriteOutcome::Applied,
    }
}

#[cfg(test)]
#[path = "fold_constants_tests.rs"]
mod tests;

//! E5-3.1, E5-3.1a: removes the `Join` nodes whose dataset contributes no column, filter or policy.

use menura_representations::logical_plan::{Operation, OutputSchema, PlanNode};

use crate::column_reads::{ColumnNames, add_operation_reads};
use crate::context::RewriteContext;
use crate::record::{NodePath, RewriteOutcome, RewriteRecord, RewriteRule, RewriteSubject};

/// Applies the rewrite to `plan` and adds a record of every `Join` it removed to `records`,
/// the last `Join` of a chain first (E5-3.1a). A used `Join` is not recorded: the rewrite is
/// considered only for an unused one, and is then never rejected (E5-3.8). It needs no
/// catalog state.
///
/// The joins under a `Snapshot` are not considered: the rows it reads besides its input are
/// joined like its input (E4-7.5, E4-7.6), so they must keep the same columns.
pub(super) fn rewrite(
    plan: PlanNode,
    _context: &RewriteContext<'_>,
    records: &mut Vec<RewriteRecord>,
) -> PlanNode {
    if !has_join(&plan) {
        return plan;
    }
    visit(&plan, &[], &ColumnNames::new(), records)
}

/// The rewrite of `node`, at `path` in the input plan, where `later_reads` are the columns its
/// ancestors read.
fn visit(
    node: &PlanNode,
    path: &[usize],
    later_reads: &ColumnNames,
    records: &mut Vec<RewriteRecord>,
) -> PlanNode {
    let mut reads = later_reads.clone();
    add_operation_reads(node.operation(), &mut reads);
    match node.operation() {
        Operation::Join { left, right, .. } => {
            visit_join(node, (left, right), path, (later_reads, &reads), records)
        }
        Operation::Deduplicate {
            input, identity, ..
        } => {
            let rewritten = visit(input, &child(path, 0), &reads, records);
            if has_join(input) && !has_join(&rewritten) {
                let identity = identity.iter().map(|name| name.as_str().to_owned());
                without_columns(&rewritten, &identity.collect())
            } else {
                rebuild(node, rewritten)
            }
        }
        Operation::Stitch { branches, .. } => {
            let branches = branches
                .iter()
                .enumerate()
                .map(|(position, branch)| visit(branch, &child(path, position), &reads, records))
                .collect();
            with_branches(node, branches)
        }
        Operation::Filter { input, .. }
        | Operation::SemiJoin { input, .. }
        | Operation::Aggregate { input, .. }
        | Operation::Project { input, .. }
        | Operation::Sort { input, .. }
        | Operation::Limit { input, .. } => {
            let input = visit(input, &child(path, 0), &reads, records);
            rebuild(node, input)
        }
        Operation::Scan { .. }
        | Operation::Periods { .. }
        | Operation::Population { .. }
        | Operation::Snapshot { .. } => node.clone(),
    }
}

/// The rewrite of the `Join` `node` of inputs `left` and `right`: removed when no later node
/// reads a column of `right`, else kept with its left input rewritten. `reads` are the columns
/// its ancestors read, then those with its own condition added.
fn visit_join(
    node: &PlanNode,
    (left, right): (&PlanNode, &PlanNode),
    path: &[usize],
    (later_reads, reads): (&ColumnNames, &ColumnNames),
    records: &mut Vec<RewriteRecord>,
) -> PlanNode {
    if is_read(right, later_reads) {
        let left = visit(left, &child(path, 0), reads, records);
        rebuild(node, left)
    } else {
        records.push(record(path, RewriteOutcome::Applied));
        visit(left, &child(path, 0), later_reads, records)
    }
}

/// The `Stitch` `node` with `branches` in place of its branches.
fn with_branches(node: &PlanNode, branches: Vec<PlanNode>) -> PlanNode {
    let Operation::Stitch { keys, columns, .. } = node.operation() else {
        return node.clone();
    };
    let operation = Operation::Stitch {
        branches,
        keys: keys.clone(),
        columns: columns.clone(),
    };
    PlanNode::new(operation, node.schema().clone(), node.origin().clone())
}

/// Whether a column of the joined plan `right` is among `later_reads` (E5-3.1a).
fn is_read(right: &PlanNode, later_reads: &ColumnNames) -> bool {
    right
        .schema()
        .columns()
        .iter()
        .any(|column| later_reads.contains(column.name.as_str()))
}

/// Whether the plan rooted at `node` has a `Join`.
fn has_join(node: &PlanNode) -> bool {
    matches!(node.operation(), Operation::Join { .. }) || node.inputs().into_iter().any(has_join)
}

/// `node` without the columns `removed`: in its schema and in the schemas of its chain of
/// `Filter`, `SemiJoin` and `Join` inputs down to the home `Scan`, whose `row_identity` goes
/// when it is removed (E4-4.0f).
fn without_columns(node: &PlanNode, removed: &ColumnNames) -> PlanNode {
    let schema = OutputSchema::new(
        node.schema()
            .columns()
            .iter()
            .filter(|column| !removed.contains(column.name.as_str()))
            .cloned()
            .collect(),
    );
    let operation = match node.operation() {
        Operation::Scan {
            dataset,
            row_identity,
        } => Operation::Scan {
            dataset: dataset.clone(),
            row_identity: row_identity
                .clone()
                .filter(|identity| !removed.contains(identity.as_str())),
        },
        Operation::Filter { input, .. }
        | Operation::SemiJoin { input, .. }
        | Operation::Join { left: input, .. } => {
            replace_input(node.operation(), without_columns(input, removed))
        }
        _ => node.operation().clone(),
    };
    PlanNode::new(operation, schema, node.origin().clone())
}

/// `node` with its first input replaced by `input`. A node whose output holds the columns of
/// its first input keeps only the columns that are still available (E4-3.11).
fn rebuild(node: &PlanNode, input: PlanNode) -> PlanNode {
    let operation = replace_input(node.operation(), input);
    let available: Option<ColumnNames> = match &operation {
        Operation::Filter { input, .. }
        | Operation::SemiJoin { input, .. }
        | Operation::Deduplicate { input, .. } => Some(column_names(&[input])),
        Operation::Join { left, right, .. } => Some(column_names(&[left, right])),
        _ => None,
    };
    let schema = match available {
        Some(available) => OutputSchema::new(
            node.schema()
                .columns()
                .iter()
                .filter(|column| available.contains(column.name.as_str()))
                .cloned()
                .collect(),
        ),
        None => node.schema().clone(),
    };
    PlanNode::new(operation, schema, node.origin().clone())
}

/// The names of the columns of `nodes`.
fn column_names(nodes: &[&PlanNode]) -> ColumnNames {
    nodes
        .iter()
        .flat_map(|node| node.schema().columns())
        .map(|column| column.name.as_str().to_owned())
        .collect()
}

/// `operation` with its first input, `left` for a `Join`, replaced by `input`. An operation
/// without one is returned unchanged.
fn replace_input(operation: &Operation, input: PlanNode) -> Operation {
    let input = Box::new(input);
    match operation {
        Operation::Join { .. } | Operation::SemiJoin { .. } => replace_left(operation, input),
        Operation::Filter { predicate, .. } => Operation::Filter {
            input,
            predicate: predicate.clone(),
        },
        Operation::Deduplicate {
            group, identity, ..
        } => Operation::Deduplicate {
            input,
            group: group.clone(),
            identity: identity.clone(),
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
        Operation::Scan { .. }
        | Operation::Periods { .. }
        | Operation::Population { .. }
        | Operation::Snapshot { .. }
        | Operation::Stitch { .. } => operation.clone(),
    }
}

/// The `Join` or `SemiJoin` `operation` with `input` in place of its left input. Another
/// operation is returned unchanged.
fn replace_left(operation: &Operation, input: Box<PlanNode>) -> Operation {
    match operation {
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
        Operation::SemiJoin {
            relationship,
            on,
            related_all,
            related_visible,
            ..
        } => Operation::SemiJoin {
            input,
            relationship: relationship.clone(),
            on: on.clone(),
            related_all: related_all.clone(),
            related_visible: related_visible.clone(),
        },
        _ => operation.clone(),
    }
}

/// The path of the input at `position` of the node at `path`.
fn child(path: &[usize], position: usize) -> Vec<usize> {
    let mut steps = path.to_vec();
    steps.push(position);
    steps
}

/// The record of E5-3.1 for the `Join` at `path` with `outcome`.
fn record(path: &[usize], outcome: RewriteOutcome) -> RewriteRecord {
    RewriteRecord {
        rule: RewriteRule::RemoveUnusedJoins,
        subject: RewriteSubject::Node(NodePath::new(path.to_vec())),
        outcome,
    }
}

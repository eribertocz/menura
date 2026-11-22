//! E5-4 step 4: finds each branch of the plan, so that a summary is selected for each one
//! separately.

use menura_representations::logical_plan::{Operation, PlanNode};

use super::selection::select;
use crate::context::RewriteContext;
use crate::plan_inputs::map_inputs;
use crate::record::RewriteRecord;

/// `node` with each branch `Aggregate` below its `Limit`, `Sort`, `Project` and `Stitch` nodes
/// replaced by the branch that reads its selected summary; `None` when no branch reads one.
pub(super) fn select_in_branches(
    node: &PlanNode,
    context: &RewriteContext<'_>,
    records: &mut Vec<RewriteRecord>,
) -> Option<PlanNode> {
    let operation = match node.operation() {
        Operation::Aggregate { .. } => return select(node, context, records),
        Operation::Limit { input, rows } => Operation::Limit {
            input: Box::new(select_in_branches(input, context, records)?),
            rows: *rows,
        },
        Operation::Sort { input, items } => Operation::Sort {
            input: Box::new(select_in_branches(input, context, records)?),
            items: items.clone(),
        },
        Operation::Project { input, expressions } => Operation::Project {
            input: Box::new(select_in_branches(input, context, records)?),
            expressions: expressions.clone(),
        },
        Operation::Stitch {
            branches,
            keys,
            columns,
        } => Operation::Stitch {
            branches: stitched_branches(branches, context, records)?,
            keys: keys.clone(),
            columns: columns.clone(),
        },
        Operation::Lookup { .. } => return selected_lookup(node, context, records),
        _ => return None,
    };
    Some(PlanNode::new(
        operation,
        node.schema().clone(),
        node.origin().clone(),
    ))
}

/// The `Lookup` `node` with the branches of its result rows and of its context query rewritten
/// by [`select_in_branches`]; `None` when none is. A context query is planned like a query
/// (E4-8.2, E4-8.9).
fn selected_lookup(
    node: &PlanNode,
    context: &RewriteContext<'_>,
    records: &mut Vec<RewriteRecord>,
) -> Option<PlanNode> {
    let mut selected: Vec<Option<PlanNode>> = node
        .inputs()
        .into_iter()
        .map(|input| select_in_branches(input, context, records))
        .collect();
    if selected.iter().all(Option::is_none) {
        return None;
    }
    Some(map_inputs(node, |index, input| {
        selected
            .get_mut(index)
            .and_then(Option::take)
            .unwrap_or_else(|| input.clone())
    }))
}

/// `branches` with each one rewritten by [`select_in_branches`]; `None` when none is.
fn stitched_branches(
    branches: &[PlanNode],
    context: &RewriteContext<'_>,
    records: &mut Vec<RewriteRecord>,
) -> Option<Vec<PlanNode>> {
    let selected: Vec<Option<PlanNode>> = branches
        .iter()
        .map(|branch| select_in_branches(branch, context, records))
        .collect();
    if selected.iter().all(Option::is_none) {
        return None;
    }
    Some(
        selected
            .into_iter()
            .zip(branches)
            .map(|(selected, branch)| selected.unwrap_or_else(|| branch.clone()))
            .collect(),
    )
}

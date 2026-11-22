//! Rebuilds a plan node with new inputs, so that a rewrite can change a node deep in a plan.

use menura_representations::logical_plan::{
    Operation, PlanNode, SnapshotMissing, SnapshotOver, SnapshotRule,
};

/// `node` with each of its inputs replaced by `map(index, input)`, where `index` is the
/// position of the input in `PlanNode::inputs`. The operation, schema and origin of `node` are
/// kept.
pub(crate) fn map_inputs(
    node: &PlanNode,
    mut map: impl FnMut(usize, &PlanNode) -> PlanNode,
) -> PlanNode {
    let mut operation = node.operation().clone();
    for (index, input) in inputs_mut(&mut operation).into_iter().enumerate() {
        *input = map(index, input);
    }
    PlanNode::new(operation, node.schema().clone(), node.origin().clone())
}

/// The inputs of `operation`, in the order of `PlanNode::inputs`.
fn inputs_mut(operation: &mut Operation) -> Vec<&mut PlanNode> {
    match operation {
        Operation::Scan { .. } | Operation::Periods { .. } => Vec::new(),
        Operation::Join { left, right, .. } => vec![left, right],
        Operation::SemiJoin {
            input,
            related_all,
            related_visible,
            ..
        } => vec![input, related_all, related_visible],
        Operation::Population { input, periods, .. } => vec![input, periods],
        Operation::Snapshot { input, rule } => {
            let mut inputs = vec![input.as_mut()];
            inputs.extend(rule_inputs_mut(rule));
            inputs
        }
        Operation::Stitch { branches, .. } => branches.iter_mut().collect(),
        Operation::Filter { input, .. }
        | Operation::Deduplicate { input, .. }
        | Operation::Aggregate { input, .. }
        | Operation::Project { input, .. }
        | Operation::Sort { input, .. }
        | Operation::Limit { input, .. } => vec![input],
    }
}

/// The plans `rule` reads, in the order of `SnapshotRule::inputs`.
fn rule_inputs_mut(rule: &mut SnapshotRule) -> Vec<&mut PlanNode> {
    let SnapshotOver::Time(time) = &mut rule.over else {
        return Vec::new();
    };
    let mut inputs = vec![time.expected.as_mut()];
    match &mut time.missing {
        SnapshotMissing::CarryForward { carried, .. } => inputs.push(carried),
        SnapshotMissing::Zero { added } => inputs.push(added),
        SnapshotMissing::Exclude | SnapshotMissing::Error => {}
    }
    inputs
}

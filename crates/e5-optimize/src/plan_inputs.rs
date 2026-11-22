//! Mutable access to the inputs of a plan operation, for rewrites that replace one input or
//! rebuild a node with new inputs.

use menura_model::internal_model::NormalizedName;
use menura_representations::logical_plan::{Operation, PlanNode, SnapshotMissing, SnapshotOver};

/// `node` with each of its inputs replaced by `map(index, input)`, where `index` is the
/// position of the input in `PlanNode::inputs`. The operation, schema and origin of `node` are
/// kept.
pub(crate) fn map_inputs(
    node: &PlanNode,
    mut map: impl FnMut(usize, &PlanNode) -> PlanNode,
) -> PlanNode {
    let mut operation = node.operation().clone();
    for index in 0..node.inputs().len() {
        if let Some(input) = input_mut(&mut operation, index) {
            *input = map(index, input);
        }
    }
    PlanNode::new(operation, node.schema().clone(), node.origin().clone())
}

/// The input of `operation` at `index`, in the order of `PlanNode::inputs`, or `None` when
/// the operation has no input at that index.
pub(crate) fn input_mut(operation: &mut Operation, index: usize) -> Option<&mut PlanNode> {
    match operation {
        Operation::Scan { .. } | Operation::Periods { .. } => None,
        Operation::Join { left, right, .. } | Operation::FullJoin { left, right, .. } => {
            match index {
                0 => Some(left.as_mut()),
                1 => Some(right.as_mut()),
                _ => None,
            }
        }
        Operation::SemiJoin {
            input,
            related_all,
            related_visible,
            ..
        } => match index {
            0 => Some(input.as_mut()),
            1 => Some(related_all.as_mut()),
            2 => Some(related_visible.as_mut()),
            _ => None,
        },
        Operation::Population { input, periods, .. } => match index {
            0 => Some(input.as_mut()),
            1 => Some(periods.as_mut()),
            _ => None,
        },
        Operation::Snapshot { input, rule } => match index {
            0 => Some(input.as_mut()),
            _ => snapshot_input_mut(&mut rule.over, index - 1),
        },
        Operation::Stitch { branches, .. } => branches.get_mut(index),
        Operation::Filter { input, .. }
        | Operation::Deduplicate { input, .. }
        | Operation::Aggregate { input, .. }
        | Operation::Project { input, .. }
        | Operation::PeriodRange { input, .. }
        | Operation::Sort { input, .. }
        | Operation::Limit { input, .. } => (index == 0).then_some(input.as_mut()),
    }
}

/// The plan of the snapshot rule over `over` at `index`, in the order of
/// `SnapshotRule::inputs`: `expected`, then the `carried` or `added` rows.
fn snapshot_input_mut(over: &mut SnapshotOver, index: usize) -> Option<&mut PlanNode> {
    let SnapshotOver::Time(time) = over else {
        return None;
    };
    match (index, &mut time.missing) {
        (0, _) => Some(time.expected.as_mut()),
        (1, SnapshotMissing::CarryForward { carried, .. }) => Some(carried.as_mut()),
        (1, SnapshotMissing::Zero { added }) => Some(added.as_mut()),
        _ => None,
    }
}

/// The dataset of the home `Scan` of the branch or node `node`: the first input of each node,
/// down to a `Scan`. `None` when the first inputs end without one.
pub(crate) fn home_dataset(node: &PlanNode) -> Option<&NormalizedName> {
    let mut current = node;
    loop {
        if let Operation::Scan { dataset, .. } = current.operation() {
            return Some(dataset);
        }
        current = current.inputs().into_iter().next()?;
    }
}

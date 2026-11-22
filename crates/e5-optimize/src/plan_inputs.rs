//! Mutable access to the inputs of a plan operation, for rewrites that replace one input.

use menura_representations::logical_plan::{Operation, PlanNode, SnapshotMissing, SnapshotOver};

/// The input of `operation` at `index`, in the order of `PlanNode::inputs`, or `None` when
/// the operation has no input at that index.
pub(crate) fn input_mut(operation: &mut Operation, index: usize) -> Option<&mut PlanNode> {
    match operation {
        Operation::Scan { .. } | Operation::Periods { .. } => None,
        Operation::Join { left, right, .. } => match index {
            0 => Some(left.as_mut()),
            1 => Some(right.as_mut()),
            _ => None,
        },
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

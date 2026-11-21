//! The `Join` steps along the join tree of a branch (E4-4.0c) and the fields each `Scan`
//! reads.

use menura_model::internal_model::NormalizedName;
use menura_representations::logical_plan::{JoinKind, Operation, OutputSchema, PlanNode};

use crate::error::PlanError;
use crate::layout::Layout;
use crate::scan::instance_scan;
use crate::visibility::Visibility;

/// `left` joined with the visible rows of every join instance of `layout`, in path order, one
/// left `Join` per instance (E4-6.5c). Each `Scan` reads the columns of `read` under its
/// prefix, and `visibility` places its own `Filter` and `SemiJoin` nodes over it. A `Join`
/// outputs the columns of its left input, then those of its right input, which a home row
/// without a match fills with NULL (E4-4.1).
///
/// # Errors
///
/// An internal error for a field without a type, or from `visibility`.
pub(crate) fn join_instances(
    left: PlanNode,
    layout: &Layout<'_>,
    read: &[NormalizedName],
    visibility: &Visibility<'_>,
) -> Result<PlanNode, PlanError> {
    let mut node = left;
    for instance in layout.instances() {
        let scan = instance_scan(instance.dataset, &instance.prefix, read)?;
        let right = visibility.visible_rows(scan, instance.dataset, Some(&instance.prefix))?;
        let mut columns = node.schema().columns().to_vec();
        columns.extend(right.schema().columns().iter().cloned().map(|mut column| {
            column.nullable = true;
            column
        }));
        node = PlanNode::new(
            Operation::Join {
                left: Box::new(node),
                right: Box::new(right),
                relationship: instance.relationship.clone(),
                kind: JoinKind::Left,
                on: instance.on.clone(),
                right_prefix: instance.prefix.clone(),
            },
            OutputSchema::new(columns),
            instance.origin.clone(),
        );
    }
    Ok(node)
}

/// The columns of `read` that the home `Scan` reads: those that are not the column of a
/// field of a join instance.
pub(crate) fn home_reads(layout: &Layout<'_>, read: &[NormalizedName]) -> Vec<NormalizedName> {
    read.iter()
        .filter(|name| {
            !layout.instances().iter().any(|instance| {
                instance.prefix.field(name.as_str()).is_some_and(|field| {
                    instance
                        .dataset
                        .fields
                        .iter()
                        .any(|candidate| candidate.name.as_str() == field)
                })
            })
        })
        .cloned()
        .collect()
}

/// The columns the `Join` conditions of `layout` read, both sides.
pub(crate) fn join_reads(layout: &Layout<'_>) -> Vec<NormalizedName> {
    layout
        .instances()
        .iter()
        .flat_map(|instance| &instance.on)
        .flat_map(|condition| [&condition.left, &condition.right])
        .map(|column| NormalizedName::new(column.as_str()))
        .collect()
}

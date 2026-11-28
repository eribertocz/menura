//! The `Join` steps along the join tree of a branch (E4-4.0c) and the fields each `Scan`
//! reads.

use menura_contracts::types::common::LogicalType;
use menura_model::internal_model::NormalizedName;
use menura_representations::check_shapes::VersionRange;
use menura_representations::logical_plan::{
    JoinKind, Operation, OutputSchema, PlanNode, TemporalCoverage, comparison_type,
};
use menura_representations::names::ColumnName;

use crate::columns::source_prefix;
use crate::coverage::PlacedCoverage;
use crate::error::{InvariantViolation, PlanError};
use crate::layout::Layout;
use crate::scan::instance_scan;
use crate::visibility::Visibility;

/// `left` joined with the visible rows of every join instance of `layout`, in path order, one
/// left `Join` per instance (E4-6.5c). Each `Scan` reads the columns of `read` under its
/// prefix, its source columns typed by the catalog of `visibility`, and `visibility` places
/// its own `Filter` and `SemiJoin` nodes over it. A `Join` outputs the columns of its left
/// input, then those of its right input, which a home row without a match fills with NULL
/// (E4-4.1).
///
/// # Errors
///
/// An internal error for a field without a type, for a source column the catalog does not
/// have, or from `visibility`.
pub(crate) fn join_instances(
    left: PlanNode,
    layout: &Layout<'_>,
    read: &[NormalizedName],
    visibility: &Visibility<'_>,
) -> Result<PlanNode, PlanError> {
    let mut node = left;
    for instance in layout.instances() {
        let scan = instance_scan(instance.dataset, &instance.prefix, read, visibility.catalog)?;
        let right = visibility.visible_rows(scan, instance.dataset, Some(&instance.prefix))?;
        let temporal = instance
            .temporal
            .as_ref()
            .map(|placed| coverage(placed, node.schema(), right.schema()))
            .transpose()
            .map_err(|violation| PlanError::invariant(violation, &instance.origin))?;
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
                temporal,
                right_prefix: instance.prefix.clone(),
            },
            OutputSchema::new(columns),
            instance.origin.clone(),
        );
    }
    Ok(node)
}

/// The coverage condition of `placed` over the left and right schemas of its `Join`
/// (VAL-2.8), its values compared as [`comparison_type`] states (VAL-2.9).
///
/// # Errors
///
/// [`InvariantViolation::ForeignField`] when a side does not have the column of its field.
pub(crate) fn coverage(
    placed: &PlacedCoverage,
    left: &OutputSchema,
    right: &OutputSchema,
) -> Result<TemporalCoverage, InvariantViolation> {
    let time = column_type(left, &placed.from_time)?;
    let start = column_type(right, &placed.valid_from)?;
    let end = column_type(right, &placed.valid_to)?;
    Ok(TemporalCoverage {
        from_time: placed.from_time.clone(),
        range: VersionRange {
            valid_from: placed.valid_from.clone(),
            valid_to: placed.valid_to.clone(),
            valid_to_bound: placed.valid_to_bound,
            open_end: placed.open_end.clone(),
        },
        compared_as: comparison_type([time, start, end]),
    })
}

/// The logical type of the column `name` of `schema`.
fn column_type<'schema>(
    schema: &'schema OutputSchema,
    name: &ColumnName,
) -> Result<&'schema LogicalType, InvariantViolation> {
    schema
        .columns()
        .iter()
        .find(|column| &column.name == name)
        .map(|column| &column.logical_type)
        .ok_or(InvariantViolation::ForeignField)
}

/// The columns of `read` that the home `Scan` reads: those that are not the column of a
/// field or of a source column of a join instance (E0D-3.1).
pub(crate) fn home_reads(layout: &Layout<'_>, read: &[NormalizedName]) -> Vec<NormalizedName> {
    read.iter()
        .filter(|name| {
            !layout.instances().iter().any(|instance| {
                instance.prefix.field(name.as_str()).is_some_and(|field| {
                    field.starts_with(&source_prefix(instance.dataset))
                        || instance
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

/// The columns the `Join` conditions of `layout` read, both sides, the coverage condition of a
/// temporal relationship included (VAL-2.8).
pub(crate) fn join_reads(layout: &Layout<'_>) -> Vec<NormalizedName> {
    let mut read = Vec::new();
    for instance in layout.instances() {
        for condition in &instance.on {
            read.push(NormalizedName::new(condition.left.as_str()));
            read.push(NormalizedName::new(condition.right.as_str()));
        }
        if let Some(placed) = &instance.temporal {
            for column in [&placed.from_time, &placed.valid_from, &placed.valid_to] {
                read.push(NormalizedName::new(column.as_str()));
            }
        }
    }
    read
}

//! The rows a branch reads below its `Aggregate`: whether two branches read the same rows, and
//! the plan that reads them with the columns of both (E5-3.4a).
//!
//! Two plans read the same rows when they are equal except for the columns their home `Scan`
//! projects: a projection never changes rows. The home `Scan` is the one reached through the
//! first input of every node.

use menura_model::internal_model::{InternalModel, NormalizedName};
use menura_representations::logical_plan::{Operation, OutputSchema, PlanNode, SchemaColumn};
use menura_representations::names::ColumnName;

/// Whether `first` and `other` are equal except for the columns their home `Scan` projects:
/// the same dataset and row identity, the same row-policy nodes, the same `Join` nodes in the
/// same order, the same query filter and the same `Deduplicate`. A plan whose home dataset `model` does not have never reads the same rows as another, so it
/// is never merged.
pub(super) fn same_rows(first: &PlanNode, other: &PlanNode, model: &InternalModel) -> bool {
    if first.origin() != other.origin() {
        return false;
    }
    match (first.operation(), other.operation()) {
        (Operation::Scan { .. }, Operation::Scan { .. }) => {
            first.operation() == other.operation() && known_home(first, model)
        }
        (
            Operation::Filter { input, predicate },
            Operation::Filter {
                input: other_input,
                predicate: other_predicate,
            },
        ) => predicate == other_predicate && same_rows(input, other_input, model),
        (
            Operation::Deduplicate {
                input,
                group,
                identity,
            },
            Operation::Deduplicate {
                input: other_input,
                group: other_group,
                identity: other_identity,
            },
        ) => {
            group == other_group
                && identity == other_identity
                && same_rows(input, other_input, model)
        }
        (
            Operation::Join { left, .. },
            Operation::Join {
                left: other_left, ..
            },
        )
        | (
            Operation::SemiJoin { input: left, .. },
            Operation::SemiJoin {
                input: other_left, ..
            },
        ) => same_except_first_input(first, other) && same_rows(left, other_left, model),
        _ => false,
    }
}

/// Whether the dataset of the `Scan` `scan` is one of `model`.
fn known_home(scan: &PlanNode, model: &InternalModel) -> bool {
    match scan.operation() {
        Operation::Scan { dataset, .. } => model.dataset(dataset).is_some(),
        _ => false,
    }
}

/// Whether the `Join` or `SemiJoin` nodes `first` and `other` are equal in everything but
/// their first input.
fn same_except_first_input(first: &PlanNode, other: &PlanNode) -> bool {
    match (first.operation(), other.operation()) {
        (
            Operation::Join {
                right,
                relationship,
                kind,
                on,
                right_prefix,
                ..
            },
            Operation::Join {
                right: other_right,
                relationship: other_relationship,
                kind: other_kind,
                on: other_on,
                right_prefix: other_prefix,
                ..
            },
        ) => {
            right == other_right
                && relationship == other_relationship
                && kind == other_kind
                && on == other_on
                && right_prefix == other_prefix
        }
        (
            Operation::SemiJoin {
                relationship,
                on,
                related_all,
                related_visible,
                ..
            },
            Operation::SemiJoin {
                relationship: other_relationship,
                on: other_on,
                related_all: other_all,
                related_visible: other_visible,
                ..
            },
        ) => {
            relationship == other_relationship
                && on == other_on
                && related_all == other_all
                && related_visible == other_visible
        }
        _ => false,
    }
}

/// The plan `first`, whose rows are the same as those of `other` (`same_rows`), with its home
/// `Scan` also projecting the columns that only the home `Scan` of `other` projects, and every
/// node above it passing them on before the columns it adds itself.
pub(super) fn with_columns_of(
    first: &PlanNode,
    other: &PlanNode,
    model: &InternalModel,
) -> PlanNode {
    if let Operation::Scan {
        dataset,
        row_identity,
    } = first.operation()
    {
        let Some(found) = model.dataset(dataset) else {
            return first.clone();
        };
        let fields: Vec<NormalizedName> = found
            .fields
            .iter()
            .map(|field| field.name.clone())
            .collect();
        let columns = scan_columns(
            first.schema(),
            other.schema(),
            &fields,
            row_identity.as_ref(),
        );
        return PlanNode::new(first.operation().clone(), columns, first.origin().clone());
    }
    let (Some(input), Some(other_input)) = (first_input(first), first_input(other)) else {
        return first.clone();
    };
    let new_input = with_columns_of(input, other_input, model);
    let added = first
        .schema()
        .columns()
        .iter()
        .skip(input.schema().columns().len());
    let columns = new_input
        .schema()
        .columns()
        .iter()
        .chain(added)
        .cloned()
        .collect();
    PlanNode::new(
        replaced_first_input(first.operation(), new_input),
        OutputSchema::new(columns),
        first.origin().clone(),
    )
}

/// The first input of a `Filter`, `Deduplicate`, `SemiJoin` or `Join` node: the one whose
/// columns its output starts with.
fn first_input(node: &PlanNode) -> Option<&PlanNode> {
    match node.operation() {
        Operation::Filter { input, .. }
        | Operation::Deduplicate { input, .. }
        | Operation::SemiJoin { input, .. }
        | Operation::Join { left: input, .. } => Some(input),
        _ => None,
    }
}

/// `operation` with `replacement` as its first input (`first_input`); any other operation
/// unchanged.
fn replaced_first_input(operation: &Operation, replacement: PlanNode) -> Operation {
    let mut operation = operation.clone();
    match &mut operation {
        Operation::Filter { input, .. }
        | Operation::Deduplicate { input, .. }
        | Operation::SemiJoin { input, .. }
        | Operation::Join { left: input, .. } => **input = replacement,
        _ => {}
    }
    operation
}

/// The columns of a home `Scan` that projects those of `first` and of `other`, in the order of
/// E4-4.0a: the fields, in the order of `fields`, the dataset's fields; then the source
/// columns, those of `first` before those only `other` has; then the synthesized
/// `row_identity`, when there is one.
pub(super) fn scan_columns(
    first: &OutputSchema,
    other: &OutputSchema,
    fields: &[NormalizedName],
    row_identity: Option<&ColumnName>,
) -> OutputSchema {
    let mut all: Vec<&SchemaColumn> = first.columns().iter().collect();
    for column in other.columns() {
        if !all.iter().any(|known| known.name == column.name) {
            all.push(column);
        }
    }
    let field_position = |column: &SchemaColumn| {
        fields
            .iter()
            .position(|field| field.as_str() == column.name.as_str())
    };
    let is_identity = |column: &SchemaColumn| Some(&column.name) == row_identity;
    let mut field_columns: Vec<&SchemaColumn> = all
        .iter()
        .copied()
        .filter(|column| field_position(column).is_some())
        .collect();
    field_columns.sort_by_key(|column| field_position(column));
    let sources = all
        .iter()
        .copied()
        .filter(|column| field_position(column).is_none() && !is_identity(column));
    let identity = all.iter().copied().filter(|column| is_identity(column));
    OutputSchema::new(
        field_columns
            .into_iter()
            .chain(sources)
            .chain(identity)
            .cloned()
            .collect(),
    )
}

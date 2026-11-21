//! The `Stitch` of several branches by the dimension keys (E4-3.7, E4-5).

use menura_model::expression::{AggregateCall, DistinctAggregate, SimpleAggregate};
use menura_representations::logical_plan::{
    AggregateComponent, EmptyInput, Operation, OutputSchema, PlanNode, SchemaColumn,
    StitchedComponent,
};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;

use crate::error::{InvariantViolation, PlanError};

/// The synthesized presence column of every branch (E6-4.7, E6-4.12). The branch columns are
/// `dimension_<n>` and `metric_<m>_component_<k>`, so no branch column takes this name.
const PRESENCE: &str = "menura.presence";

/// The `Stitch` of `branches`, in branch order, each ending with its `Aggregate` over the
/// same keys (E4-5). Its output is the keys, then the components of every branch in branch
/// order (E4-3.11).
///
/// A key column can be NULL when it can be NULL in a branch; a component can be NULL unless
/// both its branch value and its empty-input value are non-NULL (PLN-1).
///
/// # Errors
///
/// [`InvariantViolation::UnstitchableBranches`] when a branch does not end with its
/// `Aggregate`, or the branches have different keys or key types.
pub(crate) fn stitch(branches: Vec<PlanNode>, origin: Origin) -> Result<PlanNode, PlanError> {
    let fail = || PlanError::invariant(InvariantViolation::UnstitchableBranches, &origin);
    let mut keys: Option<Vec<SchemaColumn>> = None;
    let mut columns = Vec::new();
    let mut components = Vec::new();
    for branch in &branches {
        let (branch_keys, branch_components) = aggregate_parts(branch).ok_or_else(fail)?;
        let key_columns = branch_keys
            .iter()
            .map(|name| schema_column(branch, name).cloned())
            .collect::<Option<Vec<_>>>()
            .ok_or_else(fail)?;
        keys = Some(match keys {
            None => key_columns,
            Some(previous) => merged_keys(previous, key_columns).ok_or_else(fail)?,
        });
        for component in branch_components {
            let empty_input = empty_input(&component.call);
            let column = schema_column(branch, &component.name).ok_or_else(fail)?;
            columns.push(SchemaColumn {
                nullable: column.nullable || empty_input == EmptyInput::Null,
                ..column.clone()
            });
            components.push(StitchedComponent {
                name: component.name.clone(),
                empty_input,
            });
        }
    }
    let keys = keys.ok_or_else(fail)?;
    let key_names = keys.iter().map(|key| key.name.clone()).collect();
    let schema = OutputSchema::new(keys.into_iter().chain(columns).collect());
    Ok(PlanNode::new(
        Operation::Stitch {
            branches,
            keys: key_names,
            presence: ColumnName::new(PRESENCE),
            components,
        },
        schema,
        origin,
    ))
}

/// The key column names and the components of the `Aggregate` that `branch` ends with.
fn aggregate_parts(branch: &PlanNode) -> Option<(Vec<&ColumnName>, &[AggregateComponent])> {
    match branch.operation() {
        Operation::Aggregate {
            keys, components, ..
        } => Some((keys.iter().map(|key| &key.name).collect(), components)),
        _ => None,
    }
}

/// The column `name` of the output of `branch`.
fn schema_column<'plan>(branch: &'plan PlanNode, name: &ColumnName) -> Option<&'plan SchemaColumn> {
    branch
        .schema()
        .columns()
        .iter()
        .find(|column| &column.name == name)
}

/// The keys of two branches as one: same names and types, NULL when NULL in either. `None`
/// when they differ.
fn merged_keys(previous: Vec<SchemaColumn>, next: Vec<SchemaColumn>) -> Option<Vec<SchemaColumn>> {
    if previous.len() != next.len() {
        return None;
    }
    previous
        .into_iter()
        .zip(next)
        .map(|(left, right)| {
            let same = left.name == right.name && left.logical_type == right.logical_type;
            same.then_some(SchemaColumn {
                nullable: left.nullable || right.nullable,
                ..left
            })
        })
        .collect()
}

/// The value of `call` over no rows (E4-5.0b): `0` for `COUNT`, NULL for every other
/// aggregate (E0F-7.1).
fn empty_input(call: &AggregateCall) -> EmptyInput {
    match call {
        AggregateCall::CountRows
        | AggregateCall::Simple {
            function: SimpleAggregate::Count,
            ..
        }
        | AggregateCall::Distinct {
            function: DistinctAggregate::Count,
            ..
        } => EmptyInput::Zero,
        AggregateCall::Simple { .. }
        | AggregateCall::Distinct { .. }
        | AggregateCall::Percentile { .. } => EmptyInput::Null,
    }
}

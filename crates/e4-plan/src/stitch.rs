//! The `Stitch` of several branches by the dimension keys (E4-3.7, E4-5).

use menura_model::expression::{
    AggregateCall, DistinctAggregate, IntegerLiteral, Literal, SimpleAggregate,
};
use menura_representations::logical_plan::{
    AggregateComponent, Operation, OutputSchema, PlanNode, SchemaColumn, StitchColumn,
};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;

use crate::error::{InvariantViolation, PlanError};

/// The `Stitch` of `branches`, in branch order, each ending with its `Aggregate` over the
/// same keys (E4-5). Its output is the keys, then the components of every branch, in branch
/// order and, within a branch, in component order (E4-3.11). E6 detects a missing branch row
/// with a presence column of its own (E6-4.7).
///
/// A key column can be NULL when it can be NULL in a branch; a component can be NULL unless
/// both its branch value and its empty-input value are non-NULL (PLN-1).
///
/// # Errors
///
/// [`InvariantViolation::UnstitchableBranches`] when a branch does not end with its
/// `Aggregate`, or the branches have different keys or key types, or the empty value of a
/// component cannot be written.
pub(crate) fn stitch(branches: Vec<PlanNode>, origin: Origin) -> Result<PlanNode, PlanError> {
    let fail = || PlanError::invariant(InvariantViolation::UnstitchableBranches, &origin);
    let mut keys: Option<Vec<SchemaColumn>> = None;
    let mut columns = Vec::new();
    let mut stitched = Vec::new();
    for (position, branch) in branches.iter().enumerate() {
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
            let empty_value = empty_value(&component.call).map_err(|_| fail())?;
            let column = schema_column(branch, &component.name).ok_or_else(fail)?;
            columns.push(SchemaColumn {
                nullable: column.nullable || empty_value.is_none(),
                ..column.clone()
            });
            stitched.push(StitchColumn {
                name: component.name.clone(),
                branch: position,
                empty_value,
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
            columns: stitched,
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

/// The value of `call` over no rows (E4-5.0b): `0` for `COUNT`, and `None`, NULL, for every
/// other aggregate (E0F-7.1).
///
/// # Errors
///
/// [`InvariantViolation::UnstitchableBranches`] only if `0` were not an integer literal.
fn empty_value(call: &AggregateCall) -> Result<Option<Literal>, InvariantViolation> {
    match call {
        AggregateCall::CountRows
        | AggregateCall::Simple {
            function: SimpleAggregate::Count,
            ..
        }
        | AggregateCall::Distinct {
            function: DistinctAggregate::Count,
            ..
        } => IntegerLiteral::new("0")
            .map(|zero| Some(Literal::Integer(zero)))
            .ok_or(InvariantViolation::UnstitchableBranches),
        AggregateCall::Simple { .. }
        | AggregateCall::Distinct { .. }
        | AggregateCall::Percentile { .. } => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use menura_contracts::types::common::LogicalType;
    use menura_model::expression::{AggregateCall, Expression, Identifier, Reference};
    use menura_model::internal_model::NormalizedName;
    use menura_representations::logical_plan::{
        AggregateComponent, NamedExpression, Operation, OutputSchema, PlanNode, SchemaColumn,
    };
    use menura_representations::names::ColumnName;
    use menura_representations::origin::{Origin, QueryLocation};

    use super::stitch;
    use crate::error::{InvariantViolation, PlanError};

    fn origin() -> Origin {
        Origin::Query(QueryLocation::new("/query", None))
    }

    fn column(name: &str, logical_type: LogicalType) -> SchemaColumn {
        SchemaColumn {
            name: ColumnName::new(name),
            logical_type,
            nullable: true,
        }
    }

    fn scan(key_type: &LogicalType) -> PlanNode {
        PlanNode::new(
            Operation::Scan {
                dataset: NormalizedName::new("ORDERS"),
                row_identity: None,
            },
            OutputSchema::new(vec![column("REGION", key_type.clone())]),
            origin(),
        )
    }

    /// The `Aggregate` keyed by `dimension_1` of type `key_type`, with one `COUNT(*)` named
    /// `component`.
    fn branch(key_type: &LogicalType, component: &str) -> PlanNode {
        let key = Expression::Reference(Reference {
            qualifier: None,
            name: Identifier::delimited("REGION").expect("a name"),
        });
        PlanNode::new(
            Operation::Aggregate {
                input: Box::new(scan(key_type)),
                keys: vec![NamedExpression {
                    name: ColumnName::new("dimension_1"),
                    expression: key,
                    derived: None,
                }],
                components: vec![AggregateComponent {
                    name: ColumnName::new(component),
                    call: AggregateCall::CountRows,
                    filter: None,
                    recomposition: None,
                }],
            },
            OutputSchema::new(vec![
                column("dimension_1", key_type.clone()),
                column(component, LogicalType::Boolean),
            ]),
            origin(),
        )
    }

    fn violation(result: Result<PlanNode, PlanError>) -> InvariantViolation {
        match result {
            Err(PlanError::Invariant { violation, .. }) => violation,
            other => panic!("not an internal error: {other:?}"),
        }
    }

    #[test]
    fn e4_5_0a_branches_with_keys_of_different_types_are_unstitchable() {
        let branches = vec![
            branch(&LogicalType::Text, "metric_1_component_1"),
            branch(&LogicalType::Date, "metric_2_component_1"),
        ];
        assert_eq!(
            violation(stitch(branches, origin())),
            InvariantViolation::UnstitchableBranches
        );
    }

    #[test]
    fn e4_5_0a_a_branch_that_does_not_end_with_its_aggregate_is_unstitchable() {
        let branches = vec![
            branch(&LogicalType::Text, "metric_1_component_1"),
            scan(&LogicalType::Text),
        ];
        assert_eq!(
            violation(stitch(branches, origin())),
            InvariantViolation::UnstitchableBranches
        );
    }

    #[test]
    fn e4_5_0a_branches_with_the_same_keys_stitch() {
        let branches = vec![
            branch(&LogicalType::Text, "metric_1_component_1"),
            branch(&LogicalType::Text, "metric_2_component_1"),
        ];
        assert!(stitch(branches, origin()).is_ok());
    }
}

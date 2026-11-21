//! The step of a `Stitch` node (E4-3.7, E4-5, E6-4.7): the branches combined by their keys.
//!
//! The keys are the distinct key tuples of all branches, taken with `UNION`, which compares
//! NULLs as equal (E4-5.0a). Each branch is joined to the keys with `IS NOT DISTINCT FROM` and
//! carries a presence column, so a missing branch row is told apart from a branch row whose
//! values are NULL (E4-5.1): a present row gives its values, a missing one the empty-input
//! value of each component (E4-5.0b), a literal (E6-4.11). The steps of the branches come
//! first, in branch order (E6-4.12).

use menura_model::expression::{AggregateCall, DistinctAggregate, SimpleAggregate};
use menura_representations::logical_plan::{Operation, PlanNode};
use menura_representations::names::ColumnName;

use crate::body::SelectBody;
use crate::error::{GenerateError, InvariantViolation};
use crate::identifiers::quote;
use crate::steps::{StepWriter, column_type, error};

/// The alias of the distinct key tuples.
const KEYS: &str = "\"menura_keys\"";

/// The value of a component over no rows (E4-5.0b).
enum EmptyValue {
    /// `0`, the value of a count.
    Zero,
    /// `NULL`, the value of every other aggregate.
    Null,
}

impl<'input> StepWriter<'input> {
    /// The body of `node` when it is a `Stitch`; any other node here, a `Join`, is outside
    /// the slice of this generator (VS-1).
    pub(crate) fn stitch_node(
        &mut self,
        node: &'input PlanNode,
    ) -> Result<SelectBody, GenerateError> {
        match node.operation() {
            Operation::Stitch { branches, keys } => self.stitch(node, branches, keys),
            _ => Err(error(InvariantViolation::NodeOutsideSlice, node)),
        }
    }

    /// The steps of `branches`, in branch order, then the body of the `Stitch` `node`.
    fn stitch(
        &mut self,
        node: &'input PlanNode,
        branches: &'input [PlanNode],
        keys: &[ColumnName],
    ) -> Result<SelectBody, GenerateError> {
        let mut names = Vec::new();
        for branch in branches {
            if !keys.iter().all(|key| has_column(branch, key)) {
                return Err(error(InvariantViolation::UnknownColumn, node));
            }
            names.push(self.write(branch)?.name);
        }
        self.parameters.start_node(node.origin());
        let presence = presence_column(branches);
        let items = node
            .schema()
            .columns()
            .iter()
            .map(|column| stitched_item(node, branches, keys, &presence, &column.name))
            .collect::<Result<Vec<_>, _>>()?;
        let clauses = names
            .iter()
            .enumerate()
            .map(|(index, name)| branch_join(index, name, keys, &presence))
            .collect();
        Ok(SelectBody {
            items,
            from: format!("({}) AS {KEYS}", distinct_keys(&names, keys)),
            clauses,
        })
    }
}

/// Whether `node` produces the column `name`.
fn has_column(node: &PlanNode, name: &ColumnName) -> bool {
    node.schema()
        .columns()
        .iter()
        .any(|column| &column.name == name)
}

/// The shortest `menura.present` followed by zero or more `.` that no column of any branch
/// starts with, so that it never collides with a branch column (E6-4.12).
fn presence_column(branches: &[PlanNode]) -> String {
    let mut name = String::from("menura.present");
    while branches.iter().any(|branch| {
        branch
            .schema()
            .columns()
            .iter()
            .any(|column| column.name.as_str().starts_with(&name))
    }) {
        name.push('.');
    }
    name
}

/// The quoted alias of the branch at `index`.
fn branch_alias(index: usize) -> String {
    quote(&format!("menura_branch_{}", index + 1))
}

/// `SELECT keys FROM branch_1 UNION SELECT keys FROM branch_2 …`, or one row without keys
/// (E4-5.2).
fn distinct_keys(names: &[String], keys: &[ColumnName]) -> String {
    if keys.is_empty() {
        return "SELECT TRUE".to_owned();
    }
    let columns = keys
        .iter()
        .map(|key| quote(key.as_str()))
        .collect::<Vec<_>>()
        .join(", ");
    names
        .iter()
        .map(|name| format!("SELECT {columns} FROM {name}"))
        .collect::<Vec<_>>()
        .join(" UNION ")
}

/// `LEFT JOIN (SELECT *, TRUE AS presence FROM branch) AS alias ON keys.k IS NOT DISTINCT FROM
/// alias.k AND …`, or `ON TRUE` without keys.
fn branch_join(index: usize, name: &str, keys: &[ColumnName], presence: &str) -> String {
    let alias = branch_alias(index);
    let condition = if keys.is_empty() {
        "TRUE".to_owned()
    } else {
        keys.iter()
            .map(|key| {
                let key = quote(key.as_str());
                format!("{KEYS}.{key} IS NOT DISTINCT FROM {alias}.{key}")
            })
            .collect::<Vec<_>>()
            .join(" AND ")
    };
    format!(
        "LEFT JOIN (SELECT *, TRUE AS {} FROM {name}) AS {alias} ON {condition}",
        quote(presence)
    )
}

/// The select item of the output column `name`: a key from the distinct keys, or the value of
/// the one branch that produces it, replaced by its empty-input value when the branch has no
/// row for the key.
fn stitched_item(
    node: &PlanNode,
    branches: &[PlanNode],
    keys: &[ColumnName],
    presence: &str,
    name: &ColumnName,
) -> Result<String, GenerateError> {
    let column = quote(name.as_str());
    if keys.contains(name) {
        return Ok(format!("{KEYS}.{column} AS {column}"));
    }
    let mut owners = branches
        .iter()
        .enumerate()
        .filter(|(_, branch)| has_column(branch, name));
    let (index, branch) = owners
        .next()
        .ok_or_else(|| error(InvariantViolation::UnknownColumn, node))?;
    if owners.next().is_some() {
        return Err(error(InvariantViolation::AmbiguousStitchColumn, node));
    }
    let empty = match empty_value(branch, name)
        .ok_or_else(|| error(InvariantViolation::UnknownEmptyInputValue, node))?
    {
        EmptyValue::Zero => format!("CAST(0 AS {})", column_type(node, name.as_str())?),
        EmptyValue::Null => "NULL".to_owned(),
    };
    let alias = branch_alias(index);
    Ok(format!(
        "CASE WHEN {alias}.{} THEN {alias}.{column} ELSE {empty} END AS {column}",
        quote(presence)
    ))
}

/// The empty-input value of the component `name` of the `Aggregate` at the root of `branch`,
/// or `None` when the root is not an `Aggregate` or has no such component.
fn empty_value(branch: &PlanNode, name: &ColumnName) -> Option<EmptyValue> {
    let Operation::Aggregate { components, .. } = branch.operation() else {
        return None;
    };
    let component = components
        .iter()
        .find(|component| &component.name == name)?;
    Some(match component.call {
        AggregateCall::CountRows
        | AggregateCall::Simple {
            function: SimpleAggregate::Count,
            ..
        }
        | AggregateCall::Distinct {
            function: DistinctAggregate::Count,
            ..
        } => EmptyValue::Zero,
        AggregateCall::Simple { .. }
        | AggregateCall::Distinct { .. }
        | AggregateCall::Percentile { .. } => EmptyValue::Null,
    })
}

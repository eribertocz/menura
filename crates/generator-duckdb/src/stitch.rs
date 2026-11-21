//! The step of a `Stitch` node (E4-3.7, E4-5, E6-4.7): the branches combined by their keys.
//!
//! The keys are the distinct key tuples of all branches, taken with `UNION`, which compares
//! NULLs as equal (E4-5.0a). Each branch is joined to the keys with `IS NOT DISTINCT FROM` and
//! carries a presence column, so a missing branch row is told apart from a branch row whose
//! values are NULL (E4-5.1): a present row gives its values, a missing one the empty-input
//! value that E4 gives each column (E4-5.0b), written as a literal (E6-4.11). The steps of the
//! branches come first, in branch order (E6-4.12).

use menura_representations::logical_plan::{Operation, PlanNode, StitchColumn};
use menura_representations::names::ColumnName;

use crate::body::SelectBody;
use crate::error::{GenerateError, InvariantViolation};
use crate::identifiers::quote;
use crate::steps::{StepWriter, column_type, error};
use crate::values::literal_sql;

/// The alias of the distinct key tuples.
const KEYS: &str = "\"menura_keys\"";

impl<'input> StepWriter<'input> {
    /// The body of `node` when it is a `Stitch`; any other node here, a `Join`, is outside
    /// the slice of this generator (VS-1).
    pub(crate) fn stitch_node(
        &mut self,
        node: &'input PlanNode,
    ) -> Result<SelectBody, GenerateError> {
        match node.operation() {
            Operation::Stitch {
                branches,
                keys,
                columns,
            } => self.stitch(node, branches, keys, columns),
            _ => Err(error(InvariantViolation::NodeOutsideSlice, node)),
        }
    }

    /// The steps of `branches`, in branch order, then the body of the `Stitch` `node`: the
    /// `keys`, then `columns`, in order.
    fn stitch(
        &mut self,
        node: &'input PlanNode,
        branches: &'input [PlanNode],
        keys: &[ColumnName],
        columns: &[StitchColumn],
    ) -> Result<SelectBody, GenerateError> {
        check_output(node, keys, columns)?;
        let mut names = Vec::new();
        for branch in branches {
            if !keys.iter().all(|key| has_column(branch, key)) {
                return Err(error(InvariantViolation::UnknownColumn, node));
            }
            names.push(self.write(branch)?.name);
        }
        self.parameters.start_node(node.origin());
        let presence = presence_column(branches);
        let mut items: Vec<String> = keys
            .iter()
            .map(|key| {
                let key = quote(key.as_str());
                format!("{KEYS}.{key} AS {key}")
            })
            .collect();
        for column in columns {
            items.push(stitched_item(node, branches, &presence, column)?);
        }
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

/// Checks that the schema of `node` is `keys`, then `columns`, in order.
fn check_output(
    node: &PlanNode,
    keys: &[ColumnName],
    columns: &[StitchColumn],
) -> Result<(), GenerateError> {
    let expected = keys.iter().chain(columns.iter().map(|column| &column.name));
    let schema = node.schema().columns();
    if schema.len() == keys.len() + columns.len()
        && schema
            .iter()
            .zip(expected)
            .all(|(column, name)| &column.name == name)
    {
        Ok(())
    } else {
        Err(error(InvariantViolation::UnknownColumn, node))
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

/// The select item of `column`: the value of its branch, or its empty-input value cast to the
/// column type when the branch has no row for the key (`NULL` when it has none).
fn stitched_item(
    node: &PlanNode,
    branches: &[PlanNode],
    presence: &str,
    column: &StitchColumn,
) -> Result<String, GenerateError> {
    let branch = branches
        .get(column.branch)
        .ok_or_else(|| error(InvariantViolation::UnknownBranch, node))?;
    if !has_column(branch, &column.name) {
        return Err(error(InvariantViolation::UnknownColumn, node));
    }
    let empty = match &column.empty_value {
        Some(literal) => format!(
            "CAST({} AS {})",
            literal_sql(literal),
            column_type(node, column.name.as_str())?
        ),
        None => "NULL".to_owned(),
    };
    let alias = branch_alias(column.branch);
    let name = quote(column.name.as_str());
    Ok(format!(
        "CASE WHEN {alias}.{} THEN {alias}.{name} ELSE {empty} END AS {name}",
        quote(presence)
    ))
}

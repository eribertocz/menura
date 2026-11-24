//! The step of a `Stitch` node (E4-3.7, E4-5, E6-4.7): the branches combined by their keys.
//!
//! After the steps of the branches come one `present` step per branch, its rows with a
//! presence column, then one `keys` step: the distinct key tuples of all branches, taken with
//! `UNION`, which compares NULLs as equal (E4-5.0a). Each `present` step is joined to the keys
//! with a null-safe equality (E6-7.4); its presence column is TRUE on every row, so a missing branch row is told apart from a branch row whose
//! values are NULL (E4-5.1): a present row gives its values, a missing one the empty-input
//! value that E4 gives each column (E4-5.0b), written as a literal (E6-4.11). The steps of the
//! branches come first, in branch order (E6-4.12).

use menura_representations::logical_plan::{Operation, PlanNode, StitchColumn};
use menura_representations::names::ColumnName;

use crate::body::SelectBody;
use crate::error::{GenerateError, InvariantViolation};
use crate::identifiers::quote;
use crate::null_safe::join_equal;
use crate::steps::{StepWriter, cast_column, error};
use crate::values::literal_sql;

/// The alias of the distinct key tuples.
const KEYS: &str = "\"menura_keys\"";

impl<'input> StepWriter<'input> {
    /// The body of `node`, a `Stitch`; any other node is not one this function generates.
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
        let present: Vec<String> = branches
            .iter()
            .zip(&names)
            .map(|(branch, name)| self.push_step("present", present_rows(branch, name, &presence)))
            .collect();
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
        let (from, first) = match distinct_keys(&names, keys) {
            Some(body) => (format!("{} AS {KEYS}", self.push_step("keys", body)), 0),
            None => (
                format!("{} AS {}", first_name(node, &present)?, branch_alias(0)),
                1,
            ),
        };
        let clauses = present
            .iter()
            .enumerate()
            .skip(first)
            .map(|(index, name)| branch_join(index, name, keys))
            .collect();
        Ok(SelectBody {
            items,
            from,
            clauses,
        })
    }
}

/// The first `present` step: the `FROM` of a `Stitch` without keys.
fn first_name<'name>(
    node: &PlanNode,
    present: &'name [String],
) -> Result<&'name str, GenerateError> {
    present
        .first()
        .map(String::as_str)
        .ok_or_else(|| error(InvariantViolation::UnknownBranch, node))
}

/// The rows of the branch step `name`, each column of `branch` unchanged, with the presence
/// column `presence`, TRUE on every row (E6-4.7).
pub(crate) fn present_rows(branch: &PlanNode, name: &str, presence: &str) -> SelectBody {
    let mut items: Vec<String> = branch
        .schema()
        .columns()
        .iter()
        .map(|column| quote(column.name.as_str()))
        .collect();
    items.push(format!("TRUE AS {}", quote(presence)));
    SelectBody {
        items,
        from: name.to_owned(),
        clauses: Vec::new(),
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
pub(crate) fn has_column(node: &PlanNode, name: &ColumnName) -> bool {
    node.schema()
        .columns()
        .iter()
        .any(|column| &column.name == name)
}

/// The shortest `menura.present` followed by zero or more `.` that no column of any branch
/// starts with, so that it never collides with a branch column (E6-4.12).
pub(crate) fn presence_column(branches: &[PlanNode]) -> String {
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

/// The distinct key tuples of the branch steps `names`: `SELECT keys FROM branch_1`, then one
/// `UNION DISTINCT` per further branch, one clause per line (E6-4.9); `None` without keys
/// (E4-5.2). `ClickHouse` needs `DISTINCT` written, whatever `union_default_mode` holds.
fn distinct_keys(names: &[String], keys: &[ColumnName]) -> Option<SelectBody> {
    let (first, others) = names.split_first()?;
    if keys.is_empty() {
        return None;
    }
    let items: Vec<String> = keys.iter().map(|key| quote(key.as_str())).collect();
    let mut clauses = Vec::new();
    for name in others {
        clauses.push("UNION DISTINCT".to_owned());
        clauses.push("SELECT".to_owned());
        let last = items.len().saturating_sub(1);
        for (index, item) in items.iter().enumerate() {
            let separator = if index == last { "" } else { "," };
            clauses.push(format!("  {item}{separator}"));
        }
        clauses.push(format!("FROM {name}"));
    }
    Some(SelectBody {
        items,
        from: first.clone(),
        clauses,
    })
}

/// `LEFT JOIN present AS alias ON isNotDistinctFrom(keys.k, alias.k) AND …`, or `ON TRUE`
/// without keys.
fn branch_join(index: usize, present: &str, keys: &[ColumnName]) -> String {
    let alias = branch_alias(index);
    let condition = if keys.is_empty() {
        "TRUE".to_owned()
    } else {
        keys.iter()
            .map(|key| {
                let key = quote(key.as_str());
                join_equal(&format!("{KEYS}.{key}"), &format!("{alias}.{key}"))
            })
            .collect::<Vec<_>>()
            .join(" AND ")
    };
    format!("LEFT JOIN {present} AS {alias} ON {condition}")
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
        Some(literal) => {
            let value = literal_sql(literal)
                .ok_or_else(|| error(InvariantViolation::UnsupportedType, node))?;
            cast_column(node, column.name.as_str(), &value)?
        }
        None => "NULL".to_owned(),
    };
    let alias = branch_alias(column.branch);
    let name = quote(column.name.as_str());
    Ok(format!(
        "CASE WHEN {alias}.{} THEN {alias}.{name} ELSE {empty} END AS {name}",
        quote(presence)
    ))
}

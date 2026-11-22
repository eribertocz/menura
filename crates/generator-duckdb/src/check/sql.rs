//! Small pieces of the SQL of the checks that are not a single plan (CHK-6.3–CHK-6.5).

use menura_representations::logical_plan::PlanNode;
use menura_representations::names::ColumnName;

use crate::error::{GenerateError, InvariantViolation};
use crate::identifiers::quote;
use crate::steps::error;
use crate::values::text_sql;

/// `value AS "name"`.
pub(super) fn aliased(value: &str, name: &str) -> String {
    format!("{value} AS {}", quote(name))
}

/// The quoted column `name`, qualified by `alias` when given.
pub(super) fn column(alias: Option<&str>, name: &str) -> String {
    match alias {
        Some(alias) => format!("{alias}.{}", quote(name)),
        None => quote(name),
    }
}

/// `'violation' AS "name"`: the text column that names a violation (CHK-6.3, CHK-6.5).
pub(super) fn violation(text: &str, name: &str) -> String {
    aliased(&text_sql(text), name)
}

/// `c1 IS NOT NULL AND c2 IS NOT NULL AND …`.
pub(super) fn all_not_null(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("{value} IS NOT NULL"))
        .collect::<Vec<_>>()
        .join(" AND ")
}

/// `l1 = r1 AND l2 = r2 AND …`: equal field by field, so never true with a NULL.
pub(super) fn all_equal(left: &[String], right: &[String]) -> String {
    left.iter()
        .zip(right)
        .map(|(left, right)| format!("{left} = {right}"))
        .collect::<Vec<_>>()
        .join(" AND ")
}

/// `1, 2, …, count`.
pub(super) fn ordinals(count: usize) -> String {
    (1..=count)
        .map(|ordinal| ordinal.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// `base`, followed by as many `.` as needed so that it names none of `taken` (E6-4.12).
pub(super) fn synthesized(base: &str, taken: &[&ColumnName]) -> String {
    let mut name = base.to_owned();
    while taken.iter().any(|column| column.as_str() == name) {
        name.push('.');
    }
    name
}

/// Checks that a check has `count` result column names; an invariant violation in `node`
/// otherwise.
pub(super) fn expect_names(
    names: &[String],
    count: usize,
    node: &PlanNode,
) -> Result<(), GenerateError> {
    if names.len() == count {
        Ok(())
    } else {
        Err(error(InvariantViolation::UnknownColumn, node))
    }
}

/// The names of the columns of `rows`.
pub(super) fn column_names(rows: &PlanNode) -> Vec<&ColumnName> {
    rows.schema()
        .columns()
        .iter()
        .map(|known| &known.name)
        .collect()
}

/// The columns of `rows` named `names`, each quoted; an invariant violation in `rows` for a
/// name that is not one of its columns.
pub(super) fn known_columns(
    rows: &PlanNode,
    names: &[&ColumnName],
) -> Result<Vec<String>, GenerateError> {
    names
        .iter()
        .map(|name| {
            rows.schema()
                .columns()
                .iter()
                .any(|known| &known.name == *name)
                .then(|| quote(name.as_str()))
                .ok_or_else(|| error(InvariantViolation::UnknownColumn, rows))
        })
        .collect()
}

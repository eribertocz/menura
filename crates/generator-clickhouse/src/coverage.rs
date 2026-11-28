//! The coverage condition of a `Join` or `SemiJoin` along a temporal relationship (VAL-2.7,
//! VAL-2.8, VAL-2.22): the version's `valid_from` is not NULL and is at or before the row's
//! `from_time`, and the version is open-ended or the time is before its end. The values are
//! compared as values of their common type (VAL-2.9), never truncated to a day or converted to
//! the query time zone, and `open_end` is a literal (E6-4.11, VAL-2.21).

use menura_contracts::types::common::LogicalType;
use menura_model::internal_model::ValidToBound;
use menura_representations::logical_plan::{OutputSchema, PlanNode, TemporalCoverage};
use menura_representations::names::ColumnName;

use crate::error::{GenerateError, InvariantViolation};
use crate::identifiers::quote;
use crate::population::membership::open_ended;
use crate::steps::error;
use crate::types::cast_sql;

/// One side of the condition: the schema of its step and the SQL that qualifies its columns.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CoverageSide<'input> {
    /// The columns of the side.
    pub(crate) schema: &'input OutputSchema,
    /// The qualifier of its columns, such as `"menura_row"`.
    pub(crate) qualifier: &'input str,
}

/// The conditions, to be joined with `AND`, under which a version of `versions` covers the
/// `from_time` of a row of `rows` (VAL-2.7).
///
/// # Errors
///
/// An internal error when a side does not have its column, or a type has no `ClickHouse`
/// name.
pub(crate) fn coverage_conditions(
    node: &PlanNode,
    coverage: &TemporalCoverage,
    rows: CoverageSide<'_>,
    versions: CoverageSide<'_>,
) -> Result<Vec<String>, GenerateError> {
    let range = &coverage.range;
    let compared = |side: CoverageSide<'_>, name: &ColumnName| {
        let logical_type = column_type(node, side.schema, name)?;
        cast_to(node, coverage, logical_type, qualified(side, name))
    };
    let time = compared(rows, &coverage.from_time)?;
    let start = compared(versions, &range.valid_from)?;
    let end_type = column_type(node, versions.schema, &range.valid_to)?;
    let end = open_ended(
        node,
        &qualified(versions, &range.valid_to),
        range.open_end.as_ref(),
        end_type,
    )?;
    let end = cast_to(node, coverage, end_type, end)?;
    let before_end = match range.valid_to_bound {
        ValidToBound::Inclusive => "<=",
        ValidToBound::Exclusive => "<",
    };
    Ok(vec![
        format!("{start} IS NOT NULL"),
        format!("{start} <= {time}"),
        format!("({end} IS NULL OR {time} {before_end} {end})"),
    ])
}

/// The column `name` of `side`, qualified.
fn qualified(side: CoverageSide<'_>, name: &ColumnName) -> String {
    format!("{}.{}", side.qualifier, quote(name.as_str()))
}

/// The type of the column `name` of `schema`.
fn column_type<'schema>(
    node: &PlanNode,
    schema: &'schema OutputSchema,
    name: &ColumnName,
) -> Result<&'schema LogicalType, GenerateError> {
    schema
        .columns()
        .iter()
        .find(|column| &column.name == name)
        .map(|column| &column.logical_type)
        .ok_or_else(|| error(InvariantViolation::UnknownColumn, node))
}

/// `value`, of `logical_type`, cast to the comparison type of `coverage` when the types
/// differ.
fn cast_to(
    node: &PlanNode,
    coverage: &TemporalCoverage,
    logical_type: &LogicalType,
    value: String,
) -> Result<String, GenerateError> {
    if *logical_type == coverage.compared_as {
        return Ok(value);
    }
    cast_sql(&value, &coverage.compared_as)
        .ok_or_else(|| error(InvariantViolation::UnsupportedType, node))
}

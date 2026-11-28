//! The `temporal_orphans` check of a temporal relationship (VAL-2.16a): the rows of `from`
//! that no version of `to` covers at their `from_time` (VAL-2.7, VAL-2.8). It compares values
//! and computes no day, so it has no `time_zone` slot (CHK-7.3, VAL-2.9).

use menura_contracts::types::common::LogicalType;
use menura_representations::check_shapes::TemporalOrphansCheck;
use menura_representations::logical_plan::PlanNode;
use menura_representations::names::ColumnName;

use super::sql::{aliased, column, column_names, synthesized};
use crate::body::SelectBody;
use crate::error::{GenerateError, InvariantViolation};
use crate::population::membership::{column_type, open_ended};
use crate::steps::{StepWriter, error, pass_through};
use crate::types::cast_sql;
use menura_model::internal_model::ValidToBound;

/// The alias of a row of `from`.
const ROW: &str = "\"menura_row\"";
/// The alias of a version of `to`.
const VERSION: &str = "\"menura_version\"";

impl<'input> StepWriter<'input> {
    /// The rows of `check` with no covering version. The versions are read by a `CROSS JOIN`
    /// filtered by `WHERE`, as the population check reads overlaps, so that the join takes
    /// conditions other than equalities. Each row has a row number, so two rows
    /// with equal values are two rows; the rows are numbered in the order of every scanned
    /// column, since the numbered step is read twice and may be computed each time: only equal
    /// rows, which are interchangeable, can swap numbers between two readings.
    pub(super) fn temporal_orphans_check(
        &mut self,
        check: &TemporalOrphansCheck<'input>,
    ) -> Result<SelectBody, GenerateError> {
        let rows = check.rows;
        let written_rows = self.write(rows)?;
        let written_versions = self.write(check.versions)?;
        self.parameters.start_node(rows.origin());
        let row = synthesized("menura.row", &column_names(rows));
        let numbered = self.numbered_step(rows, &row, &written_rows.name)?;
        let conditions = covering_conditions(check)?;
        let covered = self.push_step(
            "covered",
            SelectBody {
                items: vec![aliased(&column(Some(ROW), &row), &row)],
                from: format!("{numbered} AS {ROW}"),
                clauses: vec![
                    format!("CROSS JOIN {} AS {VERSION}", written_versions.name),
                    format!("WHERE {}", conditions.join(" AND ")),
                    "GROUP BY 1".to_owned(),
                ],
            },
        );
        let items = check
            .result_columns
            .iter()
            .map(|(name, result)| known(rows, name).map(|value| aliased(&value, result)))
            .collect::<Result<Vec<_>, _>>()?;
        let row = column(None, &row);
        Ok(SelectBody {
            items,
            from: numbered,
            clauses: vec![format!("WHERE {row} NOT IN (SELECT {row} FROM {covered})")],
        })
    }

    /// The step of every row of `rows` with its row number as `row`.
    fn numbered_step(
        &mut self,
        rows: &'input PlanNode,
        row: &str,
        written: &str,
    ) -> Result<String, GenerateError> {
        let mut items = pass_through(rows, rows.schema())?;
        let order = column_names(rows)
            .iter()
            .map(|name| column(None, name.as_str()))
            .collect::<Vec<_>>()
            .join(", ");
        items.push(aliased(
            &format!("row_number() OVER (ORDER BY {order})"),
            row,
        ));
        Ok(self.push_step(
            "rows",
            SelectBody {
                items,
                from: written.to_owned(),
                clauses: Vec::new(),
            },
        ))
    }
}

/// The conditions under which a version covers a row (VAL-2.7, VAL-2.8): every key pair is
/// equal, the version's `valid_from` is not NULL and is at or before the row's time, and the
/// version is open-ended or the time is before its end. Values are compared as
/// `check.compared_as` (VAL-2.9).
fn covering_conditions(check: &TemporalOrphansCheck<'_>) -> Result<Vec<String>, GenerateError> {
    let (rows, versions) = (check.rows, check.versions);
    let mut conditions = Vec::new();
    for (from, to) in &check.keys {
        known(rows, from)?;
        known(versions, to)?;
        conditions.push(format!(
            "{} = {}",
            column(Some(ROW), from.as_str()),
            column(Some(VERSION), to.as_str())
        ));
    }
    let range = &check.range;
    let time = compared(
        check,
        rows,
        &check.from_time,
        column(Some(ROW), check.from_time.as_str()),
    )?;
    let start = compared(
        check,
        versions,
        &range.valid_from,
        column(Some(VERSION), range.valid_from.as_str()),
    )?;
    let end_type = column_type(versions, versions, &range.valid_to)?;
    let end = open_ended(
        versions,
        &column(Some(VERSION), range.valid_to.as_str()),
        range.open_end.as_ref(),
        end_type,
    )?;
    let end = cast_to(check, versions, end_type, end)?;
    let before_end = match range.valid_to_bound {
        ValidToBound::Inclusive => "<=",
        ValidToBound::Exclusive => "<",
    };
    conditions.push(format!("{start} IS NOT NULL"));
    conditions.push(format!("{start} <= {time}"));
    conditions.push(format!("({end} IS NULL OR {time} {before_end} {end})"));
    Ok(conditions)
}

/// The column `name` of `node`, read as `check.compared_as`.
fn compared(
    check: &TemporalOrphansCheck<'_>,
    node: &PlanNode,
    name: &ColumnName,
    value: String,
) -> Result<String, GenerateError> {
    let logical_type = column_type(node, node, name)?;
    cast_to(check, node, logical_type, value)
}

/// `value`, of `logical_type`, cast to `check.compared_as` when the types differ.
fn cast_to(
    check: &TemporalOrphansCheck<'_>,
    node: &PlanNode,
    logical_type: &LogicalType,
    value: String,
) -> Result<String, GenerateError> {
    if *logical_type == check.compared_as {
        return Ok(value);
    }
    cast_sql(&value, &check.compared_as)
        .ok_or_else(|| error(InvariantViolation::UnsupportedType, node))
}

/// The quoted column `name` of `node`; an invariant violation when it has no such column.
fn known(node: &PlanNode, name: &ColumnName) -> Result<String, GenerateError> {
    column_type(node, node, name).map(|_| column(None, name.as_str()))
}

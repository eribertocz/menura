//! The `ClickHouse` part of an `integrity_checks` case (VAL-2.16a, E6-7.11): every
//! `temporal_orphans` check is generated in both dialects, executed on the case's `DuckDB`
//! database and on its `ClickHouse` session, and both must return the same rows in any order,
//! compared as CS-2.4 compares cells.
//!
//! A check has no result schema, so the type of each column is read from the values `DuckDB`
//! returns: a timestamp is read as a timestamp with time zone, the only kind `ClickHouse` can
//! express (E6-7.12), and a column with no value as text.

use duckdb::Connection;
use duckdb::types::Value;
use menura_contracts::types::common::{FloatBits, IntegerBits, LogicalType, TimestampPrecision};
use menura_contracts::types::compiled_plan::{ColumnRole, ResultColumn};
use menura_contracts::types::descriptions::{CheckKind, IntegrityCheck, IntegrityChecksDocument};
use menura_core::{Dialect, Handle, integrity_checks};

use super::results::result_cells;
use super::server::Session;
use crate::cells::{Cell, actual_cell};
use crate::execution::execute_sql;
use crate::report::CaseStatus;
use crate::row_comparison::{Comparison, cells_mismatch};

/// The checks whose results the `ClickHouse` run compares with `DuckDB`'s.
const COMPARED: CheckKind = CheckKind::TemporalOrphans;

/// The `DuckDB` checks of `handle` that the `ClickHouse` run compares; empty when the model
/// has none, or when `integrity_checks` fails, which the `DuckDB` run reports.
pub(crate) fn compared_checks(handle: &Handle) -> Vec<IntegrityCheck> {
    integrity_checks(handle, Dialect::Duckdb)
        .map(|document| {
            document
                .checks
                .into_iter()
                .filter(|check| check.kind == COMPARED)
                .collect()
        })
        .unwrap_or_default()
}

/// Generates the checks of `handle` for `ClickHouse` and compares the rows of each check of
/// `duckdb_checks` on both engines.
pub(crate) fn check_parity(
    handle: &Handle,
    duckdb_checks: &[IntegrityCheck],
    duckdb: &Connection,
    session: &Session<'_>,
) -> CaseStatus {
    let clickhouse = match integrity_checks(handle, Dialect::Clickhouse) {
        Ok(document) => document,
        Err(error) => {
            return CaseStatus::Failed(format!(
                "`integrity_checks` fails for ClickHouse: {:?}",
                error.errors.first()
            ));
        }
    };
    let mismatches: Vec<String> = duckdb_checks
        .iter()
        .filter_map(|check| check_mismatch(check, &clickhouse, duckdb, session))
        .collect();
    if mismatches.is_empty() {
        CaseStatus::Passed
    } else {
        CaseStatus::Failed(mismatches.join("; "))
    }
}

/// Why the check `duckdb_check` returns other rows on `ClickHouse`, or `None`.
fn check_mismatch(
    duckdb_check: &IntegrityCheck,
    clickhouse: &IntegrityChecksDocument,
    duckdb: &Connection,
    session: &Session<'_>,
) -> Option<String> {
    let id = &duckdb_check.check_id;
    let compared = || -> Result<Option<String>, String> {
        let clickhouse_check = clickhouse
            .checks
            .iter()
            .find(|check| check.check_id == *id)
            .ok_or_else(|| "it is not generated for ClickHouse".to_owned())?;
        if !duckdb_check.parameters.is_empty() || !clickhouse_check.parameters.is_empty() {
            return Err("it has parameters, which the comparison does not bind".to_owned());
        }
        let expected = execute_sql(duckdb, &duckdb_check.sql, &[])
            .map_err(|failure| format!("it fails on DuckDB: {failure:?}"))?
            .rows;
        let schema = schema_of(&expected);
        let expected = expected
            .iter()
            .map(|row| {
                row.iter()
                    .zip(&schema)
                    .map(|(value, column)| actual_cell(value, &column.logical_type))
                    .collect::<Result<Vec<Cell>, String>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let output = session
            .execute(&clickhouse_check.sql, &[])
            .map_err(|failure| format!("it fails on ClickHouse: {}", failure.describe()))?;
        let actual = if expected.is_empty() && output.is_empty() {
            Vec::new()
        } else {
            result_cells(&schema, &output)?
        };
        Ok(cells_mismatch(&actual, &expected, Comparison::any_order()))
    };
    compared()
        .unwrap_or_else(Some)
        .map(|reason| format!("the check {id}: {reason}"))
}

/// The columns of the rows `rows` returns, each typed by its first non-NULL value.
fn schema_of(rows: &[Vec<Value>]) -> Vec<ResultColumn> {
    let width = rows.first().map_or(0, Vec::len);
    (0..width)
        .map(|index| {
            let logical_type = rows
                .iter()
                .filter_map(|row| row.get(index))
                .find_map(value_type)
                .unwrap_or(LogicalType::Text);
            ResultColumn {
                name: format!("column_{index}"),
                role: ColumnRole::Dimension,
                logical_type,
                physical_type: String::new(),
                nullable: true,
            }
        })
        .collect()
}

/// The logical type a `DuckDB` value is compared as; `None` for NULL.
fn value_type(value: &Value) -> Option<LogicalType> {
    let integer = LogicalType::Integer {
        bits: IntegerBits::SixtyFour,
        signed: true,
    };
    Some(match value {
        Value::Null => return None,
        Value::Boolean(_) => LogicalType::Boolean,
        Value::TinyInt(_)
        | Value::SmallInt(_)
        | Value::Int(_)
        | Value::BigInt(_)
        | Value::HugeInt(_)
        | Value::UTinyInt(_)
        | Value::USmallInt(_)
        | Value::UInt(_)
        | Value::UBigInt(_)
        | Value::UHugeInt(_) => integer,
        Value::Decimal(decimal) => LogicalType::Decimal {
            precision: 38,
            scale: decimal.scale(),
        },
        Value::Float(_) | Value::Double(_) => LogicalType::Float {
            bits: FloatBits::SixtyFour,
        },
        Value::Date32(_) => LogicalType::Date,
        Value::Timestamp(..) => LogicalType::Timestamp {
            precision: TimestampPrecision::Us,
            with_time_zone: true,
        },
        _ => LogicalType::Text,
    })
}

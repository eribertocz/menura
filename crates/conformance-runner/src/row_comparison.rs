//! Compares the rows a plan returns with the expected `rows` of a case (CS-2.4): NULLs equal,
//! integers and decimals exactly, floats within `tolerance` (relative), every other value by its
//! CON-4 text, and the row order as declared.

use duckdb::types::Value;
use menura_contracts::types::compiled_plan::ResultColumn;

use crate::case::{Expectation, RowOrder};
use crate::cells::{Cell, actual_cell, describe_rows, expected_cell};
use crate::csv_rows::{CsvField, CsvRows};
use crate::execution::StatementResult;

/// How expected and actual values are compared.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Comparison {
    /// Whether the rows must come in the same order.
    pub(crate) row_order: RowOrder,
    /// The relative tolerance of float columns; floats are compared exactly without one.
    pub(crate) tolerance: Option<f64>,
}

impl Comparison {
    /// The comparison a case declares: its `row_order`, `any` when omitted, and its `tolerance`.
    pub(crate) fn of(expect: &Expectation) -> Self {
        Self {
            row_order: expect.row_order.unwrap_or(RowOrder::Any),
            tolerance: expect.tolerance,
        }
    }
}

/// Why the actual rows differ from the expected ones, or `None` when they match.
pub(crate) fn rows_mismatch(
    schema: &[ResultColumn],
    actual: &[Vec<Value>],
    expected: &CsvRows,
    comparison: Comparison,
) -> Option<String> {
    let names: Vec<&str> = schema.iter().map(|column| column.name.as_str()).collect();
    if names != expected.header {
        return Some(format!(
            "the output names are [{}], expected [{}]",
            names.join(", "),
            expected.header.join(", ")
        ));
    }
    let actual = match cells_of_actual(schema, actual) {
        Ok(actual) => actual,
        Err(reason) => return Some(reason),
    };
    let expected = match cells_of_expected(schema, &expected.rows) {
        Ok(expected) => expected,
        Err(reason) => return Some(reason),
    };
    cells_mismatch(&actual, &expected, comparison)
}

/// Why the rows of a plan differ from those its oracle returns (CS-4.7), or `None` when they
/// match under CS-2.4. Both are read by the logical types of the plan's `result_schema`.
pub(crate) fn rows_divergence(
    schema: &[ResultColumn],
    actual: &[Vec<Value>],
    oracle: &StatementResult,
    comparison: Comparison,
) -> Option<String> {
    let names: Vec<&str> = schema.iter().map(|column| column.name.as_str()).collect();
    if names != oracle.names {
        return Some(format!(
            "the output names are [{}], the oracle's are [{}]",
            names.join(", "),
            oracle.names.join(", ")
        ));
    }
    let actual = match cells_of_actual(schema, actual) {
        Ok(actual) => actual,
        Err(reason) => return Some(reason),
    };
    let oracle_rows = match cells_of_actual(schema, &oracle.rows) {
        Ok(rows) => rows,
        Err(reason) => return Some(format!("the oracle: {reason}")),
    };
    cells_mismatch(&actual, &oracle_rows, comparison).map(|reason| {
        format!(
            "{reason}; the oracle returns {}",
            describe_rows(&oracle_rows)
        )
    })
}

/// Why the actual cells differ from the expected ones, or `None` when they match.
fn cells_mismatch(
    actual: &[Vec<Cell>],
    expected: &[Vec<Cell>],
    comparison: Comparison,
) -> Option<String> {
    if actual.len() != expected.len() {
        return Some(format!(
            "the plan returns {} rows, expected {}: {}",
            actual.len(),
            expected.len(),
            describe_rows(actual)
        ));
    }
    let matched = match comparison.row_order {
        RowOrder::Exact => actual
            .iter()
            .zip(expected)
            .all(|(row, wanted)| rows_equal(row, wanted, comparison.tolerance)),
        RowOrder::Any => same_multiset(actual, expected, comparison.tolerance),
    };
    (!matched).then(|| format!("the plan returns the rows {}", describe_rows(actual)))
}

/// Whether every expected row matches its own actual row.
fn same_multiset(actual: &[Vec<Cell>], expected: &[Vec<Cell>], tolerance: Option<f64>) -> bool {
    let mut unused: Vec<&Vec<Cell>> = actual.iter().collect();
    expected.iter().all(|wanted| {
        let found = unused
            .iter()
            .position(|row| rows_equal(row, wanted, tolerance));
        found.map(|index| unused.swap_remove(index)).is_some()
    })
}

fn rows_equal(actual: &[Cell], expected: &[Cell], tolerance: Option<f64>) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(value, wanted)| cells_equal(value, wanted, tolerance))
}

fn cells_equal(actual: &Cell, expected: &Cell, tolerance: Option<f64>) -> bool {
    match (actual, expected, tolerance) {
        (Cell::Float(value), Cell::Float(wanted), Some(tolerance)) => {
            (value - wanted).abs() <= tolerance * value.abs().max(wanted.abs())
        }
        _ => actual == expected,
    }
}

fn cells_of_actual(schema: &[ResultColumn], rows: &[Vec<Value>]) -> Result<Vec<Vec<Cell>>, String> {
    rows.iter()
        .map(|row| {
            if row.len() != schema.len() {
                return Err(format!(
                    "the plan returns rows of {} columns, but its result schema has {}",
                    row.len(),
                    schema.len()
                ));
            }
            row.iter()
                .zip(schema)
                .map(|(value, column)| actual_cell(value, &column.logical_type))
                .collect()
        })
        .collect()
}

fn cells_of_expected(
    schema: &[ResultColumn],
    rows: &[Vec<CsvField>],
) -> Result<Vec<Vec<Cell>>, String> {
    rows.iter()
        .zip(2_usize..)
        .map(|(row, line)| {
            if row.len() != schema.len() {
                return Err(format!(
                    "line {line} of the rows file has {} fields, expected {}",
                    row.len(),
                    schema.len()
                ));
            }
            row.iter()
                .zip(schema)
                .map(|(field, column)| {
                    expected_cell(field.as_deref(), &column.logical_type)
                        .map_err(|reason| format!("line {line} of the rows file: {reason}"))
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use duckdb::types::{Decimal, TimeUnit};
    use menura_contracts::types::common::{
        FloatBits, IntegerBits, LogicalType, TimestampPrecision,
    };
    use menura_contracts::types::compiled_plan::ColumnRole;

    use super::*;
    use crate::csv_rows::parse_csv;

    fn column(name: &str, logical_type: LogicalType) -> ResultColumn {
        ResultColumn {
            name: name.to_owned(),
            role: ColumnRole::Dimension,
            logical_type,
            physical_type: String::new(),
            nullable: true,
        }
    }

    fn schema() -> Vec<ResultColumn> {
        vec![
            column("region", LogicalType::Text),
            column(
                "total",
                LogicalType::Decimal {
                    precision: 38,
                    scale: 2,
                },
            ),
            column(
                "rate",
                LogicalType::Float {
                    bits: FloatBits::SixtyFour,
                },
            ),
        ]
    }

    fn row(region: Option<&str>, total: i128, rate: Option<f64>) -> Vec<Value> {
        vec![
            region.map_or(Value::Null, |text| Value::Text(text.to_owned())),
            Value::Decimal(Decimal::new(38, 2, total).unwrap()),
            rate.map_or(Value::Null, Value::Double),
        ]
    }

    const ANY: Comparison = Comparison {
        row_order: RowOrder::Any,
        tolerance: None,
    };

    fn mismatch(actual: &[Vec<Value>], csv: &str, comparison: Comparison) -> Option<String> {
        rows_mismatch(&schema(), actual, &parse_csv(csv).unwrap(), comparison)
    }

    #[test]
    fn cs_2_4_nulls_are_equal_and_decimals_are_compared_exactly() {
        let actual = [row(None, 4500, None), row(Some("N"), 3075, Some(0.5))];
        let csv = "region,total,rate\nN,30.75,0.5\n,45,\n";
        assert_eq!(mismatch(&actual, csv, ANY), None);
        assert!(mismatch(&actual, "region,total,rate\nN,30.7,0.5\n,45,\n", ANY).is_some());
    }

    #[test]
    fn cs_2_4_floats_are_compared_within_the_relative_tolerance() {
        let actual = [row(Some("N"), 0, Some(1.0 / 3.0))];
        let csv = "region,total,rate\nN,0,0.333333333\n";
        assert!(mismatch(&actual, csv, ANY).is_some());
        let tolerant = Comparison {
            tolerance: Some(1.0e-6),
            ..ANY
        };
        assert_eq!(mismatch(&actual, csv, tolerant), None);
    }

    #[test]
    fn cs_2_4_exact_order_compares_rows_in_sequence() {
        let actual = [row(Some("A"), 1, None), row(Some("B"), 2, None)];
        let csv = "region,total,rate\nB,0.02,\nA,0.01,\n";
        assert_eq!(mismatch(&actual, csv, ANY), None);
        let exact = Comparison {
            row_order: RowOrder::Exact,
            ..ANY
        };
        assert!(mismatch(&actual, csv, exact).is_some());
    }

    #[test]
    fn cs_2_4_rows_are_compared_as_a_multiset() {
        let actual = [row(Some("A"), 1, None), row(Some("A"), 1, None)];
        let csv = "region,total,rate\nA,0.01,\nB,0.01,\n";
        assert!(mismatch(&actual, csv, ANY).is_some());
    }

    #[test]
    fn cs_2_4_the_header_must_name_the_outputs_in_order() {
        let actual = [row(Some("A"), 1, None)];
        let reason = mismatch(&actual, "total,region,rate\n0.01,A,\n", ANY).unwrap();
        assert!(reason.contains("output names"), "{reason}");
    }

    #[test]
    fn cs_2_4_a_different_row_count_fails() {
        assert!(mismatch(&[], "region,total,rate\nA,1,\n", ANY).is_some());
        assert_eq!(mismatch(&[], "region,total,rate\n", ANY), None);
    }

    fn oracle(names: &[&str], rows: Vec<Vec<Value>>) -> StatementResult {
        StatementResult {
            names: names.iter().map(|name| (*name).to_owned()).collect(),
            rows,
        }
    }

    const NAMES: [&str; 3] = ["region", "total", "rate"];

    #[test]
    fn cs_4_7_rows_equal_to_the_oracle_s_under_cs_2_4_do_not_diverge() {
        let actual = [row(Some("A"), 100, Some(0.5)), row(None, 4500, None)];
        let rows = vec![row(None, 4500, None), row(Some("A"), 100, Some(0.5))];
        assert_eq!(
            rows_divergence(&schema(), &actual, &oracle(&NAMES, rows), ANY),
            None
        );
    }

    #[test]
    fn cs_4_7_other_rows_than_the_oracle_s_diverge() {
        let actual = [row(Some("A"), 100, None)];
        let rows = vec![row(Some("A"), 101, None)];
        let reason = rows_divergence(&schema(), &actual, &oracle(&NAMES, rows), ANY).unwrap();
        assert!(reason.contains("the oracle returns"), "{reason}");
    }

    #[test]
    fn cs_4_7_other_output_names_than_the_oracle_s_diverge() {
        let actual = [row(Some("A"), 100, None)];
        let rows = vec![row(Some("A"), 100, None)];
        let names = ["region", "amount", "rate"];
        let reason = rows_divergence(&schema(), &actual, &oracle(&names, rows), ANY).unwrap();
        assert!(reason.contains("output names"), "{reason}");
    }

    #[test]
    fn cs_4_7_the_oracle_rows_are_compared_in_the_declared_order() {
        let actual = [row(Some("A"), 1, None), row(Some("B"), 2, None)];
        let rows = vec![row(Some("B"), 2, None), row(Some("A"), 1, None)];
        let exact = Comparison {
            row_order: RowOrder::Exact,
            ..ANY
        };
        let oracle = oracle(&NAMES, rows);
        assert_eq!(rows_divergence(&schema(), &actual, &oracle, ANY), None);
        assert!(rows_divergence(&schema(), &actual, &oracle, exact).is_some());
    }

    #[test]
    fn cs_2_4_dates_and_instants_are_compared_in_their_con_4_form() {
        let schema = [
            column("day", LogicalType::Date),
            column(
                "at",
                LogicalType::Timestamp {
                    precision: TimestampPrecision::Us,
                    with_time_zone: true,
                },
            ),
            column(
                "count",
                LogicalType::Integer {
                    bits: IntegerBits::SixtyFour,
                    signed: true,
                },
            ),
        ];
        let actual = [vec![
            Value::Date32(20_514),
            Value::Timestamp(TimeUnit::Microsecond, 0),
            Value::BigInt(3),
        ]];
        let expected = parse_csv("day,at,count\n2026-03-02,1970-01-01T00:00:00Z,3\n").unwrap();
        assert_eq!(rows_mismatch(&schema, &actual, &expected, ANY), None);
    }
}

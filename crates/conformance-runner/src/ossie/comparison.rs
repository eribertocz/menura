//! Compares the rows of a plan with the expected rows of a Layer 2 case (CS-4.6) under the rules
//! of upstream `compliance/query/README.md`, "Comparing results": columns by position with names
//! compared case-insensitively, rows as multisets and in order when the query orders them, values
//! equal regardless of type, and approximate numerics within `1e-9 * max(|a|, |e|, 1)`.

use duckdb::types::Value;
use menura_contracts::types::compiled_plan::ResultColumn;

use crate::cells::{Cell, actual_cell, describe_rows};
use crate::ossie::expected::ExpectedRows;

/// The relative tolerance of approximate numerics, also the absolute tolerance near zero.
const TOLERANCE: f64 = 1e-9;

/// Why the rows a plan with `schema` returned differ from `expected`, or `None` when they match.
/// `ordered` says whether the row order is compared too.
pub(crate) fn rows_mismatch(
    schema: &[ResultColumn],
    actual: &[Vec<Value>],
    expected: &ExpectedRows,
    ordered: bool,
) -> Option<String> {
    let names: Vec<&str> = schema.iter().map(|column| column.name.as_str()).collect();
    let same_names = names.len() == expected.columns.len()
        && names
            .iter()
            .zip(&expected.columns)
            .all(|(name, wanted)| name.eq_ignore_ascii_case(wanted));
    if !same_names {
        return Some(format!(
            "the output names are [{}], expected [{}]",
            names.join(", "),
            expected.columns.join(", ")
        ));
    }
    let actual = match cells_of(schema, actual) {
        Ok(cells) => cells,
        Err(reason) => return Some(reason),
    };
    let wanted = match cells_of(schema, &expected.rows) {
        Ok(cells) => cells,
        Err(reason) => return Some(format!("the expected rows: {reason}")),
    };
    cells_mismatch(&actual, &wanted, ordered)
}

/// The rows as cells, each value read by the logical type of its column.
fn cells_of(schema: &[ResultColumn], rows: &[Vec<Value>]) -> Result<Vec<Vec<Cell>>, String> {
    rows.iter()
        .map(|row| {
            if row.len() != schema.len() {
                return Err(format!(
                    "a row has {} columns, but the result schema has {}",
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

fn cells_mismatch(actual: &[Vec<Cell>], expected: &[Vec<Cell>], ordered: bool) -> Option<String> {
    let matched = actual.len() == expected.len()
        && if ordered {
            actual
                .iter()
                .zip(expected)
                .all(|(row, wanted)| rows_equal(row, wanted))
        } else {
            same_multiset(actual, expected)
        };
    (!matched).then(|| {
        format!(
            "the plan returns {}, expected {}",
            describe_rows(actual),
            describe_rows(expected)
        )
    })
}

/// Whether every expected row matches its own actual row.
fn same_multiset(actual: &[Vec<Cell>], expected: &[Vec<Cell>]) -> bool {
    let mut unused: Vec<&Vec<Cell>> = actual.iter().collect();
    expected.iter().all(|wanted| {
        let found = unused.iter().position(|row| rows_equal(row, wanted));
        found.map(|index| unused.swap_remove(index)).is_some()
    })
}

fn rows_equal(actual: &[Cell], expected: &[Cell]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(value, wanted)| cells_equal(value, wanted))
}

/// Two values match when both are NULL, or neither is and they are equal; a comparison with an
/// approximate numeric uses the tolerance.
fn cells_equal(actual: &Cell, expected: &Cell) -> bool {
    match (actual, expected) {
        (Cell::Float(_), _) | (_, Cell::Float(_)) => match (number(actual), number(expected)) {
            (Some(value), Some(wanted)) => {
                (value - wanted).abs() <= TOLERANCE * value.abs().max(wanted.abs()).max(1.0)
            }
            _ => false,
        },
        _ => actual == expected,
    }
}

/// The value of a numeric cell as an approximate number.
#[expect(
    clippy::cast_precision_loss,
    reason = "an exact number is compared with an approximate one only within the tolerance"
)]
fn number(cell: &Cell) -> Option<f64> {
    match cell {
        Cell::Float(value) => Some(*value),
        Cell::Exact { mantissa, scale } => {
            let exponent = i32::try_from(*scale).ok()?;
            Some(*mantissa as f64 / 10_f64.powi(exponent))
        }
        Cell::Null | Cell::Text(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use duckdb::types::Decimal;
    use menura_contracts::types::common::{FloatBits, IntegerBits, LogicalType};
    use menura_contracts::types::compiled_plan::ColumnRole;

    use super::*;

    fn column(name: &str, logical_type: LogicalType) -> ResultColumn {
        ResultColumn {
            name: name.to_owned(),
            role: ColumnRole::Metric,
            logical_type,
            physical_type: String::new(),
            nullable: true,
        }
    }

    fn schema() -> Vec<ResultColumn> {
        vec![
            column(
                "DEPTNO",
                LogicalType::Integer {
                    bits: IntegerBits::ThirtyTwo,
                    signed: true,
                },
            ),
            column(
                "avg_sal",
                LogicalType::Float {
                    bits: FloatBits::SixtyFour,
                },
            ),
        ]
    }

    fn expected(rows: Vec<Vec<Value>>) -> ExpectedRows {
        ExpectedRows {
            columns: vec!["deptno".to_owned(), "avg_sal".to_owned()],
            rows,
        }
    }

    #[test]
    fn cs_4_6_names_match_case_insensitively_and_exact_numerics_ignore_their_type() {
        let decimal = Decimal::new(5, 0, 10).unwrap();
        let actual = [vec![Value::BigInt(10), Value::Double(2.5)]];
        let wanted = expected(vec![vec![Value::Decimal(decimal), Value::Double(2.5)]]);
        assert_eq!(rows_mismatch(&schema(), &actual, &wanted, false), None);
    }

    #[test]
    fn cs_4_6_approximate_numerics_match_within_the_tolerance() {
        let actual = [vec![Value::Int(10), Value::Double(2_073.214_285_714_286)]];
        let close = expected(vec![vec![
            Value::Int(10),
            Value::Double(2_073.214_285_714_285),
        ]]);
        let far = expected(vec![vec![Value::Int(10), Value::Double(2_073.214_3)]]);
        assert_eq!(rows_mismatch(&schema(), &actual, &close, false), None);
        assert!(rows_mismatch(&schema(), &actual, &far, false).is_some());
    }

    #[test]
    fn cs_4_6_rows_match_as_multisets_and_in_order_only_when_ordered() {
        let actual = [
            vec![Value::Int(20), Value::Null],
            vec![Value::Int(10), Value::Double(1.0)],
        ];
        let wanted = expected(vec![
            vec![Value::Int(10), Value::Double(1.0)],
            vec![Value::Int(20), Value::Null],
        ]);
        assert_eq!(rows_mismatch(&schema(), &actual, &wanted, false), None);
        assert!(rows_mismatch(&schema(), &actual, &wanted, true).is_some());
    }

    #[test]
    fn cs_4_6_duplicates_count_and_null_matches_only_null() {
        let actual = [
            vec![Value::Int(10), Value::Null],
            vec![Value::Int(10), Value::Null],
        ];
        let once = expected(vec![vec![Value::Int(10), Value::Null]]);
        let zero = expected(vec![
            vec![Value::Int(10), Value::Double(0.0)],
            vec![Value::Int(10), Value::Null],
        ]);
        assert!(rows_mismatch(&schema(), &actual, &once, false).is_some());
        assert!(rows_mismatch(&schema(), &actual, &zero, false).is_some());
    }

    #[test]
    fn cs_4_6_different_output_names_do_not_match() {
        let wanted = ExpectedRows {
            columns: vec!["deptno".to_owned(), "sum_sal".to_owned()],
            rows: Vec::new(),
        };
        assert!(rows_mismatch(&schema(), &[], &wanted, false).is_some());
    }
}

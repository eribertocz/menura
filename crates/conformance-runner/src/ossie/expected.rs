//! The expected result of an Apache Ossie case: rows or a first error code (CS-4.4, CS-4.6).

use duckdb::Connection;
use duckdb::types::Value;
use serde::Deserialize;

use crate::error::RunnerErrorKind;

/// Expected rows, with the output names in query order.
#[derive(Debug, Clone, PartialEq)]
pub struct ExpectedRows {
    /// The output column names.
    pub columns: Vec<String>,
    /// The rows; each has one value per column.
    pub rows: Vec<Vec<Value>>,
}

/// What a case expects.
#[derive(Debug, Clone, PartialEq)]
pub enum ExpectedResult {
    /// The query returns these rows.
    Rows(ExpectedRows),
    /// The query fails with this first error code.
    Error(String),
}

/// The fixture a wide-table case loads (`fixture` of `expected.json`, upstream
/// `compliance/wide_table/README.md`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WideTableFixture {
    /// `retail/`, the default when `fixture` is omitted.
    #[default]
    Retail,
    /// `stores/` with one path from `store_sales` to `date_dim`.
    Stores,
    /// `stores/` with two paths from `store_sales` to `date_dim`.
    StoresAmbiguous,
}

/// The fixture and the expected result of a wide-table case.
#[derive(Debug, Clone, PartialEq)]
pub struct WideTableExpectation {
    /// The fixture to load.
    pub fixture: WideTableFixture,
    /// The expected result.
    pub result: ExpectedResult,
}

/// `expected.json` of a wide-table case. Descriptive members (`title`, `question`, `behavior`,
/// `spec`) are not read; unknown members are allowed because the suite runs unchanged (CS-3.1).
#[derive(Deserialize)]
struct ExpectedFile {
    #[serde(default)]
    fixture: WideTableFixture,
    columns: Option<Vec<String>>,
    rows: Option<Vec<Vec<serde_json::Value>>>,
    error: Option<String>,
}

/// Parses the `expected.json` of a wide-table case.
///
/// A JSON number without a fraction or an exponent is an exact numeric, any other JSON number an
/// approximate one, and `null` is SQL NULL (upstream "Comparing results").
///
/// # Errors
///
/// Returns [`RunnerErrorKind::Malformed`] when the text is not JSON of that shape, when it gives
/// both or neither of `error` and `columns` with `rows`, or when a value is not a scalar.
pub fn parse_wide_table_expectation(text: &str) -> Result<WideTableExpectation, RunnerErrorKind> {
    let file: ExpectedFile = serde_json::from_str(text)
        .map_err(|error| RunnerErrorKind::Malformed(error.to_string()))?;
    let result = match (file.error, file.columns, file.rows) {
        (Some(code), None, None) => ExpectedResult::Error(code),
        (None, Some(columns), Some(rows)) => ExpectedResult::Rows(ExpectedRows {
            columns,
            rows: rows
                .iter()
                .map(|row| row.iter().map(json_value).collect())
                .collect::<Result<_, _>>()?,
        }),
        _ => {
            return Err(RunnerErrorKind::Malformed(
                "`expected.json` needs either `error` or both `columns` and `rows`".to_owned(),
            ));
        }
    };
    Ok(WideTableExpectation {
        fixture: file.fixture,
        result,
    })
}

/// A scalar of `expected.json` as a database value.
fn json_value(value: &serde_json::Value) -> Result<Value, RunnerErrorKind> {
    match value {
        serde_json::Value::Null => Ok(Value::Null),
        serde_json::Value::Bool(boolean) => Ok(Value::Boolean(*boolean)),
        serde_json::Value::String(text) => Ok(Value::Text(text.clone())),
        serde_json::Value::Number(number) => number_value(number),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => Err(
            RunnerErrorKind::Malformed(format!("expected value `{value}` is not a scalar")),
        ),
    }
}

fn number_value(number: &serde_json::Number) -> Result<Value, RunnerErrorKind> {
    if let Some(integer) = number.as_i64() {
        return Ok(Value::BigInt(integer));
    }
    if let Some(integer) = number.as_u64() {
        return Ok(Value::HugeInt(i128::from(integer)));
    }
    number
        .as_f64()
        .map(Value::Double)
        .ok_or_else(|| RunnerErrorKind::Malformed(format!("number `{number}` is out of range")))
}

/// Runs the `expected.sql` of a Layer 2 case on the prepared fixture to obtain the expected rows
/// (CS-4.6).
///
/// # Errors
///
/// Returns the `DuckDB` error when the statement fails.
pub fn expected_rows_of(connection: &Connection, sql: &str) -> Result<ExpectedRows, duckdb::Error> {
    let mut statement = connection.prepare(sql)?;
    let rows = statement
        .query_map([], |row| {
            let count = row.as_ref().column_count();
            (0..count).map(|index| row.get::<_, Value>(index)).collect()
        })?
        .collect::<Result<Vec<Vec<Value>>, _>>()?;
    Ok(ExpectedRows {
        columns: statement.column_names(),
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cs_4_4_reads_rows_with_exact_and_approximate_numbers() {
        let text = r#"{"title": "t", "columns": ["region", "n", "x"],
                       "rows": [["EAST", 350, 1.5], [null, 0, 2e0]]}"#;
        let expectation = parse_wide_table_expectation(text).unwrap();
        assert_eq!(expectation.fixture, WideTableFixture::Retail);
        let ExpectedResult::Rows(rows) = expectation.result else {
            panic!("rows expected");
        };
        assert_eq!(rows.columns, ["region", "n", "x"]);
        assert_eq!(
            rows.rows,
            [
                vec![
                    Value::Text("EAST".to_owned()),
                    Value::BigInt(350),
                    Value::Double(1.5)
                ],
                vec![Value::Null, Value::BigInt(0), Value::Double(2.0)],
            ]
        );
    }

    #[test]
    fn cs_4_4_reads_the_expected_error_and_the_fixture() {
        let text = r#"{"fixture": "stores_ambiguous", "error": "E_AMBIGUOUS_PATH"}"#;
        let expectation = parse_wide_table_expectation(text).unwrap();
        assert_eq!(expectation.fixture, WideTableFixture::StoresAmbiguous);
        assert_eq!(
            expectation.result,
            ExpectedResult::Error("E_AMBIGUOUS_PATH".to_owned())
        );
    }

    #[test]
    fn cs_4_4_rejects_an_expectation_with_both_rows_and_an_error() {
        let text = r#"{"columns": ["n"], "rows": [[1]], "error": "E_NO_PATH"}"#;
        assert!(matches!(
            parse_wide_table_expectation(text),
            Err(RunnerErrorKind::Malformed(_))
        ));
    }

    #[test]
    fn cs_4_6_runs_expected_sql_to_obtain_the_rows() {
        let connection = Connection::open_in_memory().unwrap();
        let sql =
            "-- comment\nSELECT *\nFROM (VALUES (29025.00, 14)) AS expected (sum_sal, emp_count);";
        let rows = expected_rows_of(&connection, sql).unwrap();
        assert_eq!(rows.columns, ["sum_sal", "emp_count"]);
        assert_eq!(rows.rows.len(), 1);
        assert_eq!(rows.rows[0][1], Value::Int(14));
    }
}

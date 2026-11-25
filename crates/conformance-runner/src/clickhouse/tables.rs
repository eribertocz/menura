//! Loads the data of a case into its `ClickHouse` session (E6-7.10): the fixture is first
//! prepared in `DuckDB` as CS-4.2 says, then each relation of the catalog schema is created on
//! `ClickHouse` with the types of E6-7.2 and filled with the rows `DuckDB` holds.
//!
//! Values reach `ClickHouse` as `TabSeparated` data in the body of an `INSERT`, never inside a
//! statement. A column whose logical type has no `ClickHouse` type (E6-7.12) is not created: a
//! plan that reads it fails with `E6-01` before any SQL is written.

use std::path::Path;

use duckdb::Connection;
use duckdb::types::Value;
use menura_contracts::types::common::{FloatBits, IntegerBits, LogicalType, TimestampPrecision};
use serde::Deserialize;

use super::parameters::escape;
use super::server::{ServerFailure, Session};
use crate::execution::execute_sql;
use crate::fixture::read_text;

/// The relations of a catalog schema, with what the loader needs of each column.
#[derive(Deserialize)]
struct CatalogRelations {
    relations: Vec<Relation>,
}

#[derive(Deserialize)]
struct Relation {
    source: String,
    columns: Vec<Column>,
}

#[derive(Deserialize)]
struct Column {
    name: String,
    logical_type: LogicalType,
    nullable: bool,
}

/// What loading the fixture of a case created, so that it can be dropped afterwards.
#[derive(Debug, Default)]
pub(crate) struct LoadedFixture {
    /// The `ClickHouse` tables created, as `(database, table)`.
    tables: Vec<(String, String)>,
    /// The sources that cannot be created on `ClickHouse`, with the reason.
    pub(crate) not_created: Vec<String>,
}

/// Creates and fills on `session` every relation of the catalog schema of `model_folder`, with
/// the rows `duckdb` holds for it.
///
/// # Errors
///
/// Why the catalog schema cannot be read, or the first statement that fails.
pub(crate) fn load_fixture(
    session: &Session<'_>,
    model_folder: &Path,
    duckdb: &Connection,
) -> Result<LoadedFixture, String> {
    let path = model_folder.join("catalog-schema.json");
    let text = read_text(&path).map_err(|error| error.to_string())?;
    let catalog: CatalogRelations =
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut loaded = LoadedFixture::default();
    for relation in catalog
        .relations
        .iter()
        .filter(|relation| !is_query(&relation.source))
    {
        let parts: Vec<&str> = relation.source.split('.').collect();
        let Some((database, table)) = clickhouse_name(&parts) else {
            loaded.not_created.push(format!(
                "`{}`: ClickHouse names a table with at most two parts (E6-7.1)",
                relation.source
            ));
            continue;
        };
        let created = create_table(session, database, table, &relation.columns)
            .map_err(|failure| failure.describe())?;
        if created.is_empty() {
            loaded.not_created.push(format!(
                "`{}`: no column has a ClickHouse type (E6-7.12)",
                relation.source
            ));
            continue;
        }
        loaded
            .tables
            .push(((*database).to_owned(), (*table).to_owned()));
        let rows = duckdb_rows(duckdb, &parts, &created)?;
        let statement = format!(
            "INSERT INTO {}.{} ({}) FORMAT TabSeparated",
            quote(database),
            quote(table),
            created
                .iter()
                .map(|column| quote(&column.name))
                .collect::<Vec<_>>()
                .join(", ")
        );
        session
            .insert(&statement, &rows)
            .map_err(|failure| failure.describe())?;
    }
    Ok(loaded)
}

/// Whether a catalog `source` is a query rather than a relation name (E6-4.3): a query is not
/// copied, the relations it reads are.
fn is_query(source: &str) -> bool {
    let first_word: String = source
        .trim_start()
        .chars()
        .take_while(char::is_ascii_alphabetic)
        .collect();
    first_word.eq_ignore_ascii_case("SELECT") || first_word.eq_ignore_ascii_case("WITH")
}

/// The `ClickHouse` database and table of a source of one part, `table` in the session's
/// database `default`, or two parts, `database.table` (E6-7.1); `None` for more parts.
fn clickhouse_name<'source>(parts: &[&'source str]) -> Option<(&'source str, &'source str)> {
    match parts {
        [table] => Some((DEFAULT_DATABASE, table)),
        [database, table] => Some((database, table)),
        _ => None,
    }
}

/// The database a statement reads when a name has no database part.
const DEFAULT_DATABASE: &str = "default";

/// Drops the tables and the databases `loaded` created, so that the next case starts from an
/// empty server.
pub(crate) fn drop_fixture(session: &Session<'_>, loaded: &LoadedFixture) -> Result<(), String> {
    let mut databases: Vec<&str> = Vec::new();
    for (database, table) in &loaded.tables {
        let statement = format!(
            "DROP TABLE IF EXISTS {}.{} SYNC",
            quote(database),
            quote(table)
        );
        session
            .execute(&statement, &[])
            .map_err(|failure| failure.describe())?;
        if database != DEFAULT_DATABASE && !databases.contains(&database.as_str()) {
            databases.push(database);
        }
    }
    for database in databases {
        let statement = format!("DROP DATABASE IF EXISTS {} SYNC", quote(database));
        session
            .execute(&statement, &[])
            .map_err(|failure| failure.describe())?;
    }
    Ok(())
}

/// Creates the table `database.table` with every column that has a `ClickHouse` type, after
/// dropping any table of that name; returns the columns created, in catalog order.
fn create_table<'columns>(
    session: &Session<'_>,
    database: &str,
    table: &str,
    columns: &'columns [Column],
) -> Result<Vec<&'columns Column>, ServerFailure> {
    let typed: Vec<(&Column, String)> = columns
        .iter()
        .filter_map(|column| {
            column_type(&column.logical_type, column.nullable).map(|name| (column, name))
        })
        .collect();
    if typed.is_empty() {
        return Ok(Vec::new());
    }
    let definitions: Vec<String> = typed
        .iter()
        .map(|(column, type_name)| format!("{} {type_name}", quote(&column.name)))
        .collect();
    let name = format!("{}.{}", quote(database), quote(table));
    session.execute(
        &format!("CREATE DATABASE IF NOT EXISTS {}", quote(database)),
        &[],
    )?;
    session.execute(&format!("DROP TABLE IF EXISTS {name} SYNC"), &[])?;
    session.execute(
        &format!(
            "CREATE TABLE {name} ({}) ENGINE = Memory",
            definitions.join(", ")
        ),
        &[],
    )?;
    Ok(typed.into_iter().map(|(column, _)| column).collect())
}

/// The `ClickHouse` type of a catalog column (E6-7.2), `Nullable` when the column is, or `None`
/// when its logical type has none (E6-7.12) or is a list, which no fixture holds.
pub(crate) fn column_type(logical_type: &LogicalType, nullable: bool) -> Option<String> {
    let scalar = scalar_type(logical_type)?;
    Some(if nullable {
        format!("Nullable({scalar})")
    } else {
        scalar
    })
}

/// The `ClickHouse` name of a scalar logical type (E6-7.2).
fn scalar_type(logical_type: &LogicalType) -> Option<String> {
    Some(match logical_type {
        LogicalType::Boolean => "Bool".to_owned(),
        LogicalType::Integer { bits, signed } => {
            format!(
                "{}Int{}",
                if *signed { "" } else { "U" },
                integer_bits(*bits)
            )
        }
        LogicalType::Decimal { precision, scale } => format!("Decimal({precision}, {scale})"),
        LogicalType::Float {
            bits: FloatBits::ThirtyTwo,
        } => "Float32".to_owned(),
        LogicalType::Float {
            bits: FloatBits::SixtyFour,
        } => "Float64".to_owned(),
        LogicalType::Text | LogicalType::Binary => "String".to_owned(),
        LogicalType::Uuid => "UUID".to_owned(),
        LogicalType::Date => "Date32".to_owned(),
        LogicalType::Timestamp {
            precision,
            with_time_zone: true,
        } => format!("DateTime64({}, 'UTC')", fraction_digits(*precision)),
        LogicalType::Timestamp { .. }
        | LogicalType::Time
        | LogicalType::Interval
        | LogicalType::List { .. }
        | LogicalType::Unsupported => return None,
    })
}

fn integer_bits(bits: IntegerBits) -> u16 {
    match bits {
        IntegerBits::Eight => 8,
        IntegerBits::Sixteen => 16,
        IntegerBits::ThirtyTwo => 32,
        IntegerBits::SixtyFour => 64,
        IntegerBits::OneHundredTwentyEight => 128,
    }
}

fn fraction_digits(precision: TimestampPrecision) -> u8 {
    match precision {
        TimestampPrecision::S => 0,
        TimestampPrecision::Ms => 3,
        TimestampPrecision::Us => 6,
        TimestampPrecision::Ns => 9,
    }
}

/// The rows `duckdb` holds for the relation `parts`, as `TabSeparated` data of `columns`.
fn duckdb_rows(
    duckdb: &Connection,
    parts: &[&str],
    columns: &[&Column],
) -> Result<Vec<u8>, String> {
    let selected: Vec<String> = columns
        .iter()
        .map(|column| read_expression(column))
        .collect();
    let relation: Vec<String> = parts.iter().map(|part| quote(part)).collect();
    let statement = format!("SELECT {} FROM {}", selected.join(", "), relation.join("."));
    let result = execute_sql(duckdb, &statement, &[])
        .map_err(|failure| format!("the fixture cannot be read from DuckDB: {failure:?}"))?;
    let mut data = Vec::new();
    for row in &result.rows {
        for (index, value) in row.iter().enumerate() {
            if index > 0 {
                data.push(b'\t');
            }
            data.extend(field_of(value)?);
        }
        data.push(b'\n');
    }
    Ok(data)
}

/// The `DuckDB` expression that reads `column` as the value `ClickHouse` reads for its type:
/// an instant as its `UTC` text with every fraction digit of its precision, binary data as its
/// bytes, and any other value as text.
fn read_expression(column: &Column) -> String {
    let name = quote(&column.name);
    match column.logical_type {
        LogicalType::Timestamp {
            precision,
            with_time_zone: true,
        } => format!(
            "strftime(timezone('UTC', {name}), '%Y-%m-%d %H:%M:%S{}')",
            fraction_format(precision)
        ),
        LogicalType::Binary => name,
        _ => format!("CAST({name} AS VARCHAR)"),
    }
}

/// The `strftime` fraction of a timestamp precision: none, milliseconds, microseconds or
/// nanoseconds.
fn fraction_format(precision: TimestampPrecision) -> &'static str {
    match precision {
        TimestampPrecision::S => "",
        TimestampPrecision::Ms => ".%g",
        TimestampPrecision::Us => ".%f",
        TimestampPrecision::Ns => ".%n",
    }
}

/// The escaped `TabSeparated` field of a value read as text or as bytes.
fn field_of(value: &Value) -> Result<Vec<u8>, String> {
    match value {
        Value::Null => Ok(b"\\N".to_vec()),
        Value::Text(text) => Ok(escape(text).into_bytes()),
        Value::Blob(bytes) => Ok(escape_bytes(bytes)),
        other => Err(format!(
            "the fixture value {other:?} was not read as text or bytes"
        )),
    }
}

/// `bytes` with the escapes of `ClickHouse`'s escaped text form; every other byte is written
/// as it is.
fn escape_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut escaped = Vec::with_capacity(bytes.len());
    for byte in bytes {
        match byte {
            b'\\' => escaped.extend_from_slice(b"\\\\"),
            b'\t' => escaped.extend_from_slice(b"\\t"),
            b'\n' => escaped.extend_from_slice(b"\\n"),
            b'\r' => escaped.extend_from_slice(b"\\r"),
            0 => escaped.extend_from_slice(b"\\0"),
            other => escaped.push(*other),
        }
    }
    escaped
}

/// `name` as a quoted identifier, with every double quote doubled. It is valid in both engines.
fn quote(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(logical_type: LogicalType, nullable: bool) -> Column {
        Column {
            name: "c".to_owned(),
            logical_type,
            nullable,
        }
    }

    #[test]
    fn e6_7_10_copies_columns_with_the_e6_7_2_types() {
        let integer = LogicalType::Integer {
            bits: IntegerBits::ThirtyTwo,
            signed: false,
        };
        assert_eq!(column_type(&integer, false).as_deref(), Some("UInt32"));
        let decimal = LogicalType::Decimal {
            precision: 10,
            scale: 2,
        };
        assert_eq!(
            column_type(&decimal, true).as_deref(),
            Some("Nullable(Decimal(10, 2))")
        );
        let instant = LogicalType::Timestamp {
            precision: TimestampPrecision::Us,
            with_time_zone: true,
        };
        assert_eq!(
            column_type(&instant, false).as_deref(),
            Some("DateTime64(6, 'UTC')")
        );
        assert_eq!(
            column_type(&LogicalType::Date, false).as_deref(),
            Some("Date32")
        );
    }

    #[test]
    fn e6_7_12_creates_no_column_for_a_type_without_a_clickhouse_type() {
        let wall_clock = LogicalType::Timestamp {
            precision: TimestampPrecision::Us,
            with_time_zone: false,
        };
        assert_eq!(column_type(&wall_clock, false), None);
        assert_eq!(column_type(&LogicalType::Time, true), None);
        assert_eq!(column_type(&LogicalType::Unsupported, true), None);
    }

    #[test]
    fn e6_7_10_copies_a_source_under_the_same_database_and_table() {
        assert_eq!(
            clickhouse_name(&["main", "orders"]),
            Some(("main", "orders"))
        );
        assert_eq!(clickhouse_name(&["orders"]), Some(("default", "orders")));
        assert_eq!(clickhouse_name(&["shop", "main", "orders"]), None);
    }

    #[test]
    fn e6_7_10_copies_no_query_source() {
        assert!(is_query("SELECT order_id FROM shop.orders"));
        assert!(is_query("  with x AS (SELECT 1) SELECT * FROM x"));
        assert!(!is_query("shop.orders"));
        assert!(!is_query("selections.orders"));
    }

    #[test]
    fn e6_7_10_reads_an_instant_of_duckdb_in_utc_with_its_precision() {
        let instant = LogicalType::Timestamp {
            precision: TimestampPrecision::Us,
            with_time_zone: true,
        };
        assert_eq!(
            read_expression(&column(instant, false)),
            "strftime(timezone('UTC', \"c\"), '%Y-%m-%d %H:%M:%S.%f')"
        );
        let nanoseconds = LogicalType::Timestamp {
            precision: TimestampPrecision::Ns,
            with_time_zone: true,
        };
        assert!(read_expression(&column(nanoseconds, false)).ends_with(".%n')"));
    }

    #[test]
    fn e6_7_10_copies_binary_values_as_their_bytes() {
        assert_eq!(
            read_expression(&column(LogicalType::Binary, false)),
            "\"c\""
        );
        assert_eq!(
            read_expression(&column(LogicalType::Text, false)),
            "CAST(\"c\" AS VARCHAR)"
        );
    }

    #[test]
    fn e6_7_10_writes_fixture_values_as_escaped_fields() {
        assert_eq!(field_of(&Value::Null).unwrap(), b"\\N");
        assert_eq!(field_of(&Value::Text("a\tb".to_owned())).unwrap(), b"a\\tb");
        assert_eq!(
            field_of(&Value::Blob(vec![0xff, b'\t', 0, b'\\'])).unwrap(),
            b"\xff\\t\\0\\\\"
        );
        assert!(field_of(&Value::Int(1)).is_err());
    }
}

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
    for relation in &catalog.relations {
        let parts: Vec<&str> = relation.source.split('.').collect();
        let [database, table] = parts.as_slice() else {
            loaded.not_created.push(format!(
                "`{}`: ClickHouse names a table with two parts, `database.table` (E6-7.1)",
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
        if database != "default" && !databases.contains(&database.as_str()) {
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
        .filter_map(|column| column_type(column).map(|name| (column, name)))
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
fn column_type(column: &Column) -> Option<String> {
    let scalar = scalar_type(&column.logical_type)?;
    Some(if column.nullable {
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

/// The rows `duckdb` holds for the relation `parts`, as `TabSeparated` data of `columns`: each
/// value is read as text, an instant in `UTC`.
fn duckdb_rows(duckdb: &Connection, parts: &[&str], columns: &[&Column]) -> Result<String, String> {
    let selected: Vec<String> = columns.iter().map(|column| text_of(column)).collect();
    let relation: Vec<String> = parts.iter().map(|part| quote(part)).collect();
    let statement = format!("SELECT {} FROM {}", selected.join(", "), relation.join("."));
    let result = execute_sql(duckdb, &statement, &[])
        .map_err(|failure| format!("the fixture cannot be read from DuckDB: {failure:?}"))?;
    let mut data = String::new();
    for row in &result.rows {
        let fields: Vec<String> = row.iter().map(field_of).collect::<Result<_, _>>()?;
        data.push_str(&fields.join("\t"));
        data.push('\n');
    }
    Ok(data)
}

/// The `DuckDB` expression that reads `column` as the text `ClickHouse` reads for its type.
fn text_of(column: &Column) -> String {
    let name = quote(&column.name);
    match column.logical_type {
        LogicalType::Timestamp {
            with_time_zone: true,
            ..
        } => format!("strftime(timezone('UTC', {name}), '%Y-%m-%d %H:%M:%S.%f')"),
        _ => format!("CAST({name} AS VARCHAR)"),
    }
}

/// The escaped `TabSeparated` field of a value read as text.
fn field_of(value: &Value) -> Result<String, String> {
    match value {
        Value::Null => Ok("\\N".to_owned()),
        Value::Text(text) => Ok(escape(text)),
        other => Err(format!("the fixture value {other:?} was not read as text")),
    }
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
    fn e6_7_2_creates_columns_with_the_clickhouse_types() {
        let integer = LogicalType::Integer {
            bits: IntegerBits::ThirtyTwo,
            signed: false,
        };
        assert_eq!(
            column_type(&column(integer, false)).as_deref(),
            Some("UInt32")
        );
        let decimal = LogicalType::Decimal {
            precision: 10,
            scale: 2,
        };
        assert_eq!(
            column_type(&column(decimal, true)).as_deref(),
            Some("Nullable(Decimal(10, 2))")
        );
        let instant = LogicalType::Timestamp {
            precision: TimestampPrecision::Us,
            with_time_zone: true,
        };
        assert_eq!(
            column_type(&column(instant, false)).as_deref(),
            Some("DateTime64(6, 'UTC')")
        );
        assert_eq!(
            column_type(&column(LogicalType::Date, false)).as_deref(),
            Some("Date32")
        );
    }

    #[test]
    fn e6_7_12_creates_no_column_for_a_type_without_a_clickhouse_type() {
        let wall_clock = LogicalType::Timestamp {
            precision: TimestampPrecision::Us,
            with_time_zone: false,
        };
        assert_eq!(column_type(&column(wall_clock, false)), None);
        assert_eq!(column_type(&column(LogicalType::Time, true)), None);
        assert_eq!(column_type(&column(LogicalType::Unsupported, true)), None);
    }

    #[test]
    fn e6_7_10_reads_an_instant_of_duckdb_in_utc() {
        let instant = LogicalType::Timestamp {
            precision: TimestampPrecision::Us,
            with_time_zone: true,
        };
        assert_eq!(
            text_of(&column(instant, false)),
            "strftime(timezone('UTC', \"c\"), '%Y-%m-%d %H:%M:%S.%f')"
        );
        assert_eq!(
            text_of(&column(LogicalType::Text, false)),
            "CAST(\"c\" AS VARCHAR)"
        );
    }

    #[test]
    fn e6_7_10_writes_fixture_values_as_escaped_fields() {
        assert_eq!(field_of(&Value::Null).unwrap(), "\\N");
        assert_eq!(field_of(&Value::Text("a\tb".to_owned())).unwrap(), "a\\tb");
        assert!(field_of(&Value::Int(1)).is_err());
    }
}

//! Checks the tables the runner copied into `ClickHouse` (E6-7.10) against the case's catalog
//! schema: each column's type, as `system.columns` reports it, is mapped as E6-7.13 states, and
//! the result must equal what E6-7.13 gives for the E6-7.2 type of the catalog column, which is
//! the catalog type itself, except `binary`, which E6-7.2 writes as `String` and E6-7.13 reads
//! back as `text`.

use menura_contracts::types::common::LogicalType;

use super::parameters::escape;
use super::results::unescape;
use super::server::Session;
use super::tables::column_type;
use super::type_mapping::{MappedType, map_clickhouse_type};

/// The statement that lists the columns of one table, in their order, with bound parameters.
const COLUMNS_STATEMENT: &str = "SELECT name, type FROM system.columns \
     WHERE database = {database:String} AND table = {table:String} ORDER BY position";

/// A table the runner created on `ClickHouse`, with the catalog columns copied into it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CopiedTable {
    /// The `ClickHouse` database.
    pub(crate) database: String,
    /// The `ClickHouse` table.
    pub(crate) table: String,
    /// The columns created, in catalog order.
    pub(crate) columns: Vec<CopiedColumn>,
}

/// A catalog column copied into a [`CopiedTable`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CopiedColumn {
    /// The column name.
    pub(crate) name: String,
    /// The logical type the catalog schema gives it.
    pub(crate) logical_type: LogicalType,
    /// Whether the catalog schema says it admits NULL.
    pub(crate) nullable: bool,
}

/// Why the first table of `tables` does not read back from `session` as the catalog schema
/// describes it, or `None` when every table does.
///
/// # Errors
///
/// Why `system.columns` cannot be read.
pub(crate) fn copy_mismatch(
    session: &Session<'_>,
    tables: &[CopiedTable],
) -> Result<Option<String>, String> {
    for table in tables {
        let parameters = [
            ("database".to_owned(), escape(&table.database)),
            ("table".to_owned(), escape(&table.table)),
        ];
        let text = session
            .execute(COLUMNS_STATEMENT, &parameters)
            .map_err(|failure| failure.describe())?;
        let read_back = read_columns(&text)?;
        if let Some(mismatch) = table_mismatch(table, &read_back) {
            return Ok(Some(mismatch));
        }
    }
    Ok(None)
}

/// The `(name, type)` rows of the `TabSeparated` output of [`COLUMNS_STATEMENT`].
fn read_columns(text: &str) -> Result<Vec<(String, String)>, String> {
    text.lines()
        .map(|line| {
            line.split_once('\t')
                .map(|(name, physical_type)| (unescape(name), unescape(physical_type)))
                .ok_or_else(|| format!("`system.columns` returns the row `{line}`"))
        })
        .collect()
}

/// Why `read_back`, the columns `ClickHouse` reports for `table`, differ from the catalog
/// columns copied into it; the message names the first column that differs.
fn table_mismatch(table: &CopiedTable, read_back: &[(String, String)]) -> Option<String> {
    let qualified = |column: &str| format!("`{}.{}.{column}`", table.database, table.table);
    let copied_names: Vec<&str> = table
        .columns
        .iter()
        .map(|column| column.name.as_str())
        .collect();
    let read_names: Vec<&str> = read_back.iter().map(|(name, _)| name.as_str()).collect();
    if copied_names != read_names {
        return Some(format!(
            "the table `{}.{}` reads back from ClickHouse with the columns {read_names:?}, \
             not the copied columns {copied_names:?} (E6-7.10)",
            table.database, table.table
        ));
    }
    table
        .columns
        .iter()
        .zip(read_back)
        .find_map(|(column, (_, physical_type))| {
            let mapped = map_clickhouse_type(physical_type);
            let expected = expected_mapping(column);
            (mapped != expected).then(|| {
                format!(
                    "the column {} reads back from ClickHouse as `{physical_type}`, which \
                     E6-7.13 maps to {}; the catalog schema requires {} (E6-7.10)",
                    qualified(&column.name),
                    describe(&mapped),
                    describe(&expected)
                )
            })
        })
}

/// What E6-7.13 gives for the E6-7.2 type of `column`, computed from the very writer the copy
/// uses, so that the two cannot drift: its own logical type and nullability, except that
/// `binary`, written `String`, reads back as `text`. A column without an E6-7.2 type is never
/// copied; should one appear, it is expected as `unsupported`, which no copied type maps to.
fn expected_mapping(column: &CopiedColumn) -> MappedType {
    match column_type(&column.logical_type, column.nullable) {
        Some(written) => map_clickhouse_type(&written),
        None => MappedType {
            logical_type: LogicalType::Unsupported,
            nullable: column.nullable,
        },
    }
}

/// A mapped type as its CAT-4.1 JSON form and its nullability.
fn describe(mapped: &MappedType) -> String {
    let logical_type = serde_json::to_string(&mapped.logical_type)
        .unwrap_or_else(|_| format!("{:?}", mapped.logical_type));
    format!("`{logical_type}` with `nullable: {}`", mapped.nullable)
}

#[cfg(test)]
mod tests {
    use menura_contracts::types::common::IntegerBits;

    use super::*;

    fn copied(name: &str, logical_type: LogicalType, nullable: bool) -> CopiedColumn {
        CopiedColumn {
            name: name.to_owned(),
            logical_type,
            nullable,
        }
    }

    fn orders(columns: Vec<CopiedColumn>) -> CopiedTable {
        CopiedTable {
            database: "shop".to_owned(),
            table: "orders".to_owned(),
            columns,
        }
    }

    fn read(columns: &[(&str, &str)]) -> Vec<(String, String)> {
        columns
            .iter()
            .map(|(name, physical_type)| ((*name).to_owned(), (*physical_type).to_owned()))
            .collect()
    }

    fn integer() -> LogicalType {
        LogicalType::Integer {
            bits: IntegerBits::SixtyFour,
            signed: true,
        }
    }

    #[test]
    fn e6_7_13_accepts_tables_that_read_back_as_the_catalog_schema() {
        let table = orders(vec![
            copied("id", integer(), false),
            copied("note", LogicalType::Text, true),
            copied("payload", LogicalType::Binary, false),
        ]);
        let read_back = read(&[
            ("id", "Int64"),
            ("note", "Nullable(String)"),
            ("payload", "String"),
        ]);
        assert_eq!(table_mismatch(&table, &read_back), None);
    }

    #[test]
    fn e6_7_13_names_the_column_whose_type_differs() {
        let table = orders(vec![
            copied("id", integer(), false),
            copied("note", LogicalType::Text, false),
        ]);
        let read_back = read(&[("id", "Int64"), ("note", "Nullable(String)")]);
        let message = table_mismatch(&table, &read_back).unwrap();
        assert!(message.contains("`shop.orders.note`"), "{message}");
        assert!(message.contains("`Nullable(String)`"), "{message}");
    }

    #[test]
    fn e6_7_13_names_the_columns_when_they_differ() {
        let table = orders(vec![copied("id", integer(), false)]);
        let message = table_mismatch(&table, &read(&[("other", "Int64")])).unwrap();
        assert!(message.contains("`shop.orders`"), "{message}");
        assert!(message.contains("\"other\""), "{message}");
    }

    #[test]
    fn e6_7_13_reads_the_rows_of_system_columns() {
        assert_eq!(
            read_columns("id\tInt64\nnote\\tx\tNullable(String)\n").unwrap(),
            read(&[("id", "Int64"), ("note\tx", "Nullable(String)")])
        );
        assert!(read_columns("broken\n").is_err());
    }
}

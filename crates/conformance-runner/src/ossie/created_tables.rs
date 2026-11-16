//! Reads the tables a fixture created, so the catalog schema can be built from them (CS-4.4).

use duckdb::Connection;

/// A column of a created table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedColumn {
    /// The column name.
    pub name: String,
    /// The `DuckDB` type, as `information_schema.columns.data_type` gives it.
    pub physical_type: String,
    /// Whether the column accepts NULL.
    pub nullable: bool,
}

/// A table created by a fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedRelation {
    /// `schema.table` in the default database, `database.schema.table` otherwise (CS-4.2).
    pub source: String,
    /// The columns, in table order.
    pub columns: Vec<CreatedColumn>,
}

const CREATED_COLUMNS: &str = "\
SELECT table_catalog = current_database(), table_catalog, table_schema, table_name, column_name, \
       data_type, is_nullable = 'YES' \
FROM information_schema.columns \
WHERE table_schema NOT IN ('information_schema', 'pg_catalog') \
  AND table_catalog NOT IN ('system', 'temp') \
ORDER BY table_catalog, table_schema, table_name, ordinal_position";

/// Every user table of the database, sorted by database, schema and table name.
///
/// # Errors
///
/// Returns the `DuckDB` error when the catalog cannot be read.
pub fn read_created_tables(connection: &Connection) -> Result<Vec<CreatedRelation>, duckdb::Error> {
    let mut statement = connection.prepare(CREATED_COLUMNS)?;
    let columns = statement
        .query_map([], |row| {
            let in_default: bool = row.get(0)?;
            let database: String = row.get(1)?;
            let schema: String = row.get(2)?;
            let table: String = row.get(3)?;
            let source = if in_default {
                format!("{schema}.{table}")
            } else {
                format!("{database}.{schema}.{table}")
            };
            let column = CreatedColumn {
                name: row.get(4)?,
                physical_type: row.get(5)?,
                nullable: row.get(6)?,
            };
            Ok((source, column))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(group_by_source(columns))
}

/// Groups consecutive columns of the same source into one relation.
fn group_by_source(columns: Vec<(String, CreatedColumn)>) -> Vec<CreatedRelation> {
    let mut relations: Vec<CreatedRelation> = Vec::new();
    for (source, column) in columns {
        match relations.last_mut() {
            Some(relation) if relation.source == source => relation.columns.push(column),
            _ => relations.push(CreatedRelation {
                source,
                columns: vec![column],
            }),
        }
    }
    relations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cs_4_4_reads_the_tables_the_fixture_created() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE SCHEMA retail; \
                 CREATE TABLE retail.orders (order_id VARCHAR NOT NULL, amount DECIMAL(10, 2)); \
                 CREATE TABLE retail.customers (customer_id VARCHAR NOT NULL);",
            )
            .unwrap();
        let relations = read_created_tables(&connection).unwrap();
        let sources: Vec<&str> = relations.iter().map(|r| r.source.as_str()).collect();
        assert_eq!(sources, ["retail.customers", "retail.orders"]);
        assert_eq!(
            relations[1].columns,
            [
                CreatedColumn {
                    name: "order_id".to_owned(),
                    physical_type: "VARCHAR".to_owned(),
                    nullable: false,
                },
                CreatedColumn {
                    name: "amount".to_owned(),
                    physical_type: "DECIMAL(10,2)".to_owned(),
                    nullable: true,
                },
            ]
        );
    }

    #[test]
    fn cs_4_2_names_attached_tables_with_their_database() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("ATTACH ':memory:' AS shop; CREATE TABLE shop.main.orders (id INTEGER);")
            .unwrap();
        let relations = read_created_tables(&connection).unwrap();
        assert_eq!(relations[0].source, "shop.main.orders");
    }
}

//! Prepares the fresh in-memory `DuckDB` database of one case (CS-4.2).

use duckdb::{Config, Connection};

use crate::fixture::ModelFixture;

/// Opens a fresh in-memory database with the non-default session settings of CS-4.2, attaches
/// an in-memory database for every three-part source, then runs `data.sql`.
///
/// Extensions are never installed or loaded automatically, so a run never reaches the network.
/// The non-UTC `TimeZone` of CS-4.2 needs the ICU extension, which the bundled `DuckDB` build
/// does not contain; it is blocked by issue #23 and not set yet.
///
/// # Errors
///
/// Returns the `DuckDB` error of the first statement that fails.
pub fn prepare_database(fixture: &ModelFixture) -> Result<Connection, duckdb::Error> {
    let config = Config::default()
        .enable_autoload_extension(false)?
        .with("autoinstall_known_extensions", "false")?;
    let connection = Connection::open_in_memory_with_flags(config)?;
    apply_session_settings(&connection)?;
    for name in fixture.attached_databases() {
        connection.execute_batch(&format!("ATTACH ':memory:' AS {};", quote_identifier(name)))?;
    }
    connection.execute_batch(&fixture.data_script)?;
    Ok(connection)
}

/// Turns on `integer_division` (CS-4.2).
fn apply_session_settings(connection: &Connection) -> Result<(), duckdb::Error> {
    connection.execute_batch("SET integer_division = true;")
}

/// A `DuckDB` identifier in double quotes. Identifiers cannot be bound as parameters, so the
/// fixture's database name is quoted instead, with embedded quotes doubled.
fn quote_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_fixture() -> ModelFixture {
        ModelFixture {
            sources: Vec::new(),
            data_script: String::new(),
        }
    }

    #[test]
    fn cs_4_2_extensions_are_never_installed_automatically() {
        let connection = prepare_database(&empty_fixture()).unwrap();
        let install: bool = connection
            .query_row(
                "SELECT current_setting('autoinstall_known_extensions')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!install);
    }

    #[test]
    fn cs_4_2_session_uses_integer_division() {
        let connection = prepare_database(&empty_fixture()).unwrap();
        let setting: bool = connection
            .query_row("SELECT current_setting('integer_division')", [], |row| {
                row.get(0)
            })
            .unwrap();
        let quotient: i64 = connection
            .query_row("SELECT 7 / 2", [], |row| row.get(0))
            .unwrap();
        assert!(setting);
        assert_eq!(quotient, 3);
    }

    #[test]
    fn cs_4_2_attaches_three_part_databases_before_data_sql() {
        let fixture = ModelFixture {
            sources: vec!["shop.main.orders".to_owned(), "main.notes".to_owned()],
            data_script: "CREATE TABLE shop.main.orders (id INTEGER); \
                          CREATE TABLE main.notes (id INTEGER); \
                          INSERT INTO shop.main.orders VALUES (1), (2);"
                .to_owned(),
        };
        let connection = prepare_database(&fixture).unwrap();
        let count: i64 = connection
            .query_row("SELECT count(*) FROM shop.main.orders", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn cs_4_2_every_case_gets_a_fresh_database() {
        let fixture = ModelFixture {
            sources: Vec::new(),
            data_script: "CREATE TABLE main.orders (id INTEGER);".to_owned(),
        };
        prepare_database(&fixture).unwrap();
        prepare_database(&fixture).unwrap();
    }

    #[test]
    fn quote_identifier_doubles_embedded_quotes() {
        assert_eq!(quote_identifier("a\"b"), "\"a\"\"b\"");
    }
}

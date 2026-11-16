//! Prepares the fresh in-memory `DuckDB` database of one case (CS-4.2).

use duckdb::{Config, Connection};

use crate::fixture::ModelFixture;

/// The session time zone. It is not UTC, to prove that generated SQL does not depend on the
/// session time zone (CS-4.2, E6-4.10).
pub const SESSION_TIME_ZONE: &str = "America/La_Paz";

/// Opens a fresh in-memory database with the non-default session settings of CS-4.2, attaches
/// an in-memory database for every three-part source, then runs `data.sql`.
///
/// Extensions are never downloaded: the time zone support (ICU) is linked into the runner.
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

/// Sets the session time zone to [`SESSION_TIME_ZONE`] and turns on `integer_division`.
fn apply_session_settings(connection: &Connection) -> Result<(), duckdb::Error> {
    connection.execute_batch(&format!(
        "SET TimeZone = {}; SET integer_division = true;",
        quote_literal(SESSION_TIME_ZONE)
    ))
}

/// A `DuckDB` identifier in double quotes. Identifiers cannot be bound as parameters, so the
/// fixture's database name is quoted instead, with embedded quotes doubled.
fn quote_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// A `DuckDB` string literal for the constant time zone name; `SET` takes no bound parameters.
fn quote_literal(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
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
    fn cs_4_2_session_time_zone_is_not_utc() {
        let connection = prepare_database(&empty_fixture()).unwrap();
        let time_zone: String = connection
            .query_row("SELECT current_setting('TimeZone')", [], |row| row.get(0))
            .unwrap();
        assert_eq!(time_zone, SESSION_TIME_ZONE);
        assert_ne!(time_zone, "UTC");
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

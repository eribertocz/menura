//! Prepares the fresh in-memory `DuckDB` database of one case (CS-4.2).

use duckdb::{Config, Connection};

use crate::fixture::ModelFixture;

/// Opens a fresh in-memory database with the non-default session settings of CS-4.2, attaches
/// an in-memory database for every three-part source, then runs `data.sql`.
///
/// Extensions are never installed or loaded automatically, so a run never reaches the network.
/// The non-UTC `TimeZone` of CS-4.2 comes from the `icu` extension built into the prebuilt
/// `DuckDB` library the runner links against (issue #23).
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

/// The version of the `DuckDB` library the runner links against, as `pragma_version()` reports
/// it (for example `v1.2.0`). The compatibility matrix records it (CI-4.3).
///
/// # Errors
///
/// Returns the `DuckDB` error when the in-memory database cannot be opened or queried.
pub fn library_version() -> Result<String, duckdb::Error> {
    Connection::open_in_memory()?.query_row(
        "SELECT library_version FROM pragma_version()",
        [],
        |row| row.get(0),
    )
}

/// The session time zone of CS-4.2. It is not UTC and has no daylight saving time, so the
/// offset is the same for every timestamp.
const SESSION_TIME_ZONE: &str = "America/La_Paz";

/// Sets the non-default session settings of CS-4.2: a non-UTC `TimeZone` and
/// `integer_division`.
fn apply_session_settings(connection: &Connection) -> Result<(), duckdb::Error> {
    connection.execute_batch(&format!(
        "SET TimeZone = '{SESSION_TIME_ZONE}'; SET integer_division = true;"
    ))
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
    fn ci_4_3_reports_the_exact_version_of_the_linked_library() {
        let version = library_version().unwrap();
        assert!(version.starts_with('v'), "{version}");
        assert_eq!(version.matches('.').count(), 2, "{version}");
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

    fn setting(connection: &Connection, name: &str) -> String {
        connection
            .query_row("SELECT current_setting(?)::VARCHAR", [name], |row| {
                row.get(0)
            })
            .unwrap()
    }

    #[test]
    fn cs_4_2_session_time_zone_is_not_utc_without_automatic_extensions() {
        let connection = prepare_database(&empty_fixture()).unwrap();
        let offset_hours: i64 = connection
            .query_row(
                "SELECT date_part('timezone_hour', TIMESTAMPTZ '2024-01-15 12:00:00+00')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(setting(&connection, "TimeZone"), SESSION_TIME_ZONE);
        assert_ne!(offset_hours, 0);
        assert_eq!(
            setting(&connection, "autoinstall_known_extensions"),
            "false"
        );
        assert_eq!(setting(&connection, "autoload_known_extensions"), "false");
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

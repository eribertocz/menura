//! The time-zone data check an executor makes before it executes a compiled plan on `ClickHouse`
//! (EXE-6.1): the `TZDATA_VERSION` the server reports must be the version the compatibility
//! matrix records for the `ClickHouse` version in use (CI-4.3, E6-7.16).
//!
//! `DuckDB` gets no check: its time-zone data is fixed by its build (EXE-1.1). The compiler's own
//! version, `producer.tzdata_version`, is never compared with the engine's (EXE-6.2).

use std::fmt;

/// The code of a `ClickHouse` time-zone data version that differs or cannot be read (EXE-8).
pub const TIME_ZONE_MISMATCH_CODE: &str = "REXE-03";

/// Why a plan must not be executed on a `ClickHouse` server: its time-zone data version differs
/// from the one the compatibility matrix records, or cannot be read (`REXE-03`, EXE-6.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeZoneMismatch {
    /// The version the compatibility matrix records for the `ClickHouse` version in use.
    pub recorded_version: String,
    /// The version the server reports as `TZDATA_VERSION`, or `None` when it cannot be read.
    pub server_version: Option<String>,
}

impl TimeZoneMismatch {
    /// The code of the failure, `REXE-03` (EXE-8).
    #[must_use]
    pub fn code(&self) -> &'static str {
        TIME_ZONE_MISMATCH_CODE
    }
}

impl fmt::Display for TimeZoneMismatch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.server_version {
            Some(server) => write!(
                formatter,
                "{}: the server's time-zone data is {server}, but the compatibility matrix \
                 records {} (EXE-6.1)",
                self.code(),
                self.recorded_version
            ),
            None => write!(
                formatter,
                "{}: the server's time-zone data version cannot be read; the compatibility \
                 matrix records {} (EXE-6.1)",
                self.code(),
                self.recorded_version
            ),
        }
    }
}

impl std::error::Error for TimeZoneMismatch {}

/// Checks that the `ClickHouse` server's `TZDATA_VERSION`, `server_version` (`None` when it
/// cannot be read), is `recorded_version`, the one the compatibility matrix records for the
/// `ClickHouse` version in use (EXE-6.1).
///
/// # Errors
///
/// A [`TimeZoneMismatch`], code `REXE-03`, when the versions differ, or when the server's
/// version is `None` or empty.
///
/// # Example
///
/// ```
/// use menura_conformance_runner::time_zone_check::check_clickhouse_time_zone_data;
///
/// assert!(check_clickhouse_time_zone_data("2025b", Some("2025b")).is_ok());
/// let mismatch = check_clickhouse_time_zone_data("2025b", Some("2024a")).unwrap_err();
/// assert_eq!(mismatch.code(), "REXE-03");
/// ```
pub fn check_clickhouse_time_zone_data(
    recorded_version: &str,
    server_version: Option<&str>,
) -> Result<(), TimeZoneMismatch> {
    let server_version = server_version.filter(|version| !version.is_empty());
    if server_version == Some(recorded_version) {
        return Ok(());
    }
    Err(TimeZoneMismatch {
        recorded_version: recorded_version.to_owned(),
        server_version: server_version.map(str::to_owned),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exe_6_1_the_recorded_version_passes() {
        assert_eq!(
            check_clickhouse_time_zone_data("2025b", Some("2025b")),
            Ok(())
        );
    }

    #[test]
    fn exe_6_1_a_different_version_fails_with_rexe_03() {
        let mismatch = check_clickhouse_time_zone_data("2025b", Some("2025a")).unwrap_err();
        assert_eq!(mismatch.code(), "REXE-03");
        assert_eq!(mismatch.server_version.as_deref(), Some("2025a"));
        assert!(mismatch.to_string().starts_with("REXE-03: "));
    }

    #[test]
    fn exe_6_1_an_unreadable_version_fails_with_rexe_03() {
        for server in [None, Some("")] {
            let mismatch = check_clickhouse_time_zone_data("2025b", server).unwrap_err();
            assert_eq!(mismatch.code(), "REXE-03");
            assert_eq!(mismatch.server_version, None);
        }
    }

    #[test]
    fn exe_6_1_versions_are_compared_exactly() {
        assert!(check_clickhouse_time_zone_data("2025b", Some("2025B")).is_err());
        assert!(check_clickhouse_time_zone_data("2025b", Some(" 2025b")).is_err());
    }

    #[test]
    fn exe_6_2_the_compiler_version_is_never_compared_with_the_engine() {
        let producer = menura_core::build_producer().unwrap();
        let engine = format!("{}-other", producer.tzdata_version);
        assert_ne!(producer.tzdata_version, engine);
        assert_eq!(
            check_clickhouse_time_zone_data(&engine, Some(&engine)),
            Ok(())
        );
    }
}

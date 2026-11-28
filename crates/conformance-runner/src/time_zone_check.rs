//! The time-zone database check an executor makes before it executes a compiled plan (EXE-6.1):
//! the version the compiler used, `producer.tzdata_version` (E6-5.5), must be the version of the
//! time-zone database the engine uses.

use std::fmt;

/// The code of a time-zone database version that differs or cannot be determined (EXE-8).
pub const TIME_ZONE_MISMATCH_CODE: &str = "REXE-03";

/// Why a plan must not be executed on an engine: its time-zone database version differs from
/// the plan's, or cannot be determined (`REXE-03`, EXE-6.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeZoneMismatch {
    /// The version the plan declares, `producer.tzdata_version`.
    pub plan_version: String,
    /// The engine's version, or `None` when it cannot be determined.
    pub engine_version: Option<String>,
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
        match &self.engine_version {
            Some(engine) => write!(
                formatter,
                "{}: the plan's time-zone database is {} but the engine's is {engine} (EXE-6.1)",
                self.code(),
                self.plan_version
            ),
            None => write!(
                formatter,
                "{}: the plan's time-zone database is {} and the engine's cannot be determined \
                 (EXE-6.1)",
                self.code(),
                self.plan_version
            ),
        }
    }
}

impl std::error::Error for TimeZoneMismatch {}

/// Checks that the plan's time-zone database version `plan_version` is the engine's
/// `engine_version`, which is `None` when it cannot be determined (EXE-6.1).
///
/// # Errors
///
/// A [`TimeZoneMismatch`], code `REXE-03`, when the versions differ, or when the engine's
/// version is `None` or empty.
///
/// # Example
///
/// ```
/// use menura_conformance_runner::time_zone_check::check_time_zone_database;
///
/// assert!(check_time_zone_database("2025b", Some("2025b")).is_ok());
/// let mismatch = check_time_zone_database("2025b", Some("2024a")).unwrap_err();
/// assert_eq!(mismatch.code(), "REXE-03");
/// ```
pub fn check_time_zone_database(
    plan_version: &str,
    engine_version: Option<&str>,
) -> Result<(), TimeZoneMismatch> {
    let engine_version = engine_version.filter(|version| !version.is_empty());
    if engine_version == Some(plan_version) {
        return Ok(());
    }
    Err(TimeZoneMismatch {
        plan_version: plan_version.to_owned(),
        engine_version: engine_version.map(str::to_owned),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exe_6_1_the_same_version_passes() {
        assert_eq!(check_time_zone_database("2025b", Some("2025b")), Ok(()));
    }

    #[test]
    fn exe_6_1_a_different_version_fails_with_rexe_03() {
        let mismatch = check_time_zone_database("2025b", Some("2025a")).unwrap_err();
        assert_eq!(mismatch.code(), "REXE-03");
        assert_eq!(mismatch.engine_version.as_deref(), Some("2025a"));
        assert!(mismatch.to_string().starts_with("REXE-03: "));
    }

    #[test]
    fn exe_6_1_an_undetermined_version_fails_with_rexe_03() {
        for engine in [None, Some("")] {
            let mismatch = check_time_zone_database("2025b", engine).unwrap_err();
            assert_eq!(mismatch.code(), "REXE-03");
            assert_eq!(mismatch.engine_version, None);
        }
    }

    #[test]
    fn exe_6_1_versions_are_compared_exactly() {
        assert!(check_time_zone_database("2025b", Some("2025B")).is_err());
        assert!(check_time_zone_database("2025b", Some(" 2025b")).is_err());
    }
}

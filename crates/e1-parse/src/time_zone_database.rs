//! The version of the IANA time-zone database bundled with the compiler (E6-5.5).

/// The version of the IANA time-zone database E1 resolves time zones and relative ranges with,
/// such as `2026f`. The compiled plan's `producer` declares it (E6-5.5).
///
/// # Example
///
/// ```
/// let version = menura_e1_parse::time_zone_database_version();
/// assert!(version.starts_with("20"));
/// ```
#[must_use]
pub fn time_zone_database_version() -> &'static str {
    jiff_tzdb::VERSION
}

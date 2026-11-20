//! Known IANA time-zone names (E1-2.3).

use jiff::tz::TimeZoneDatabase;

/// Whether `name` is the exact spelling, code point by code point, of a zone or a link of the
/// IANA time-zone database bundled with the compiler. Never reads the system database.
///
/// The database lookup ignores case, so a name is known only when the zone found carries the
/// same name.
pub(crate) fn is_known_time_zone(name: &str) -> bool {
    TimeZoneDatabase::bundled()
        .get(name)
        .is_ok_and(|zone| zone.iana_name() == Some(name))
}

#[cfg(test)]
mod tests {
    use super::is_known_time_zone;

    #[test]
    fn e1_2_3_accepts_zones_and_links_by_their_exact_spelling() {
        assert!(is_known_time_zone("Europe/Madrid"));
        assert!(is_known_time_zone("US/Eastern"));
        assert!(is_known_time_zone("UTC"));
        assert!(!is_known_time_zone("europe/madrid"));
        assert!(!is_known_time_zone("Mars/Olympus"));
        assert!(!is_known_time_zone(""));
    }
}

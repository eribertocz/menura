//! Known IANA time-zone names (E1-2.3), and the date of the reference time in a zone
//! (QRY-4.2).

use jiff::civil::DateTime;
use jiff::tz::{Offset, TimeZoneDatabase};
use menura_model::expression::DateValue;
use menura_representations::time::ReferenceTime;

/// The digits of a fraction of a second in nanoseconds.
const NANOSECOND_DIGITS: usize = 9;

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

/// The date of the instant `reference` in the zone `time_zone` of the bundled database: the
/// date relative ranges count from (QRY-4.2). Returns `None` when the zone is unknown or the
/// date falls outside the years 0001–9999.
pub(crate) fn local_date(reference: &ReferenceTime, time_zone: &str) -> Option<DateValue> {
    let zone = TimeZoneDatabase::bundled().get(time_zone).ok()?;
    let offset = Offset::from_seconds(i32::from(reference.offset.minutes()) * 60).ok()?;
    let instant = offset.to_timestamp(written_date_time(reference)?).ok()?;
    let date = zone.to_datetime(instant).date();
    DateValue::new(
        u16::try_from(date.year()).ok()?,
        u8::try_from(date.month()).ok()?,
        u8::try_from(date.day()).ok()?,
    )
}

/// The date and time of `reference` as written, at its own offset.
fn written_date_time(reference: &ReferenceTime) -> Option<DateTime> {
    let date = reference.local.date;
    let time = &reference.local.time;
    let digits = time.fraction_digits();
    let padding = "0".repeat(NANOSECOND_DIGITS.checked_sub(digits.len())?);
    let nanoseconds: i32 = format!("{digits}{padding}").parse().ok()?;
    DateTime::new(
        i16::try_from(date.year()).ok()?,
        i8::try_from(date.month()).ok()?,
        i8::try_from(date.day()).ok()?,
        i8::try_from(time.hour()).ok()?,
        i8::try_from(time.minute()).ok()?,
        i8::try_from(time.second()).ok()?,
        nanoseconds,
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use menura_model::expression::DateValue;

    use super::{is_known_time_zone, local_date};
    use crate::reference_time::read_reference_time;

    #[test]
    fn e1_2_3_accepts_zones_and_links_by_their_exact_spelling() {
        assert!(is_known_time_zone("Europe/Madrid"));
        assert!(is_known_time_zone("US/Eastern"));
        assert!(is_known_time_zone("UTC"));
        assert!(!is_known_time_zone("europe/madrid"));
        assert!(!is_known_time_zone("Mars/Olympus"));
        assert!(!is_known_time_zone(""));
    }

    fn date_in(instant: &str, zone: &str) -> Option<DateValue> {
        local_date(&read_reference_time(instant).expect("an instant"), zone)
    }

    #[test]
    fn qry_4_2_the_reference_date_follows_the_zone_across_midnight_utc() {
        assert_eq!(
            date_in("2026-11-20T23:30:00Z", "Asia/Tokyo"),
            DateValue::new(2026, 11, 21)
        );
        assert_eq!(
            date_in("2026-11-20T01:00:00+02:00", "America/New_York"),
            DateValue::new(2026, 11, 19)
        );
        assert_eq!(
            date_in("2026-11-20T23:59:59.999999Z", "UTC"),
            DateValue::new(2026, 11, 20)
        );
    }

    #[test]
    fn qry_4_2_a_reference_date_after_the_year_9999_is_none() {
        assert_eq!(date_in("9999-12-31T23:00:00Z", "Pacific/Kiritimati"), None);
    }
}

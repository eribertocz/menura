//! Dates, times and timestamps in the CON-4 text forms, converted to and from the day and
//! microsecond counts `DuckDB` binds and returns.

/// Microseconds in one day.
const MICROSECONDS_PER_DAY: i64 = 86_400_000_000;

/// The days from 1970-01-01 to the proleptic Gregorian date `year-month-day`.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_index = (month + 9) % 12;
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The proleptic Gregorian date of the day `days` after 1970-01-01.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Parses a run of exactly `width` ASCII digits.
fn digits(text: &str, width: usize) -> Option<i64> {
    (text.len() == width && text.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| text.parse().ok())
        .flatten()
}

/// The days since 1970-01-01 of a date `YYYY-MM-DD`, or `None` when it is not one.
pub(crate) fn parse_date(text: &str) -> Option<i64> {
    let mut parts = text.split('-');
    let year = digits(parts.next()?, 4)?;
    let month = digits(parts.next()?, 2)?;
    let day = digits(parts.next()?, 2)?;
    if parts.next().is_some() || !(1..=12).contains(&month) || day < 1 {
        return None;
    }
    let days = days_from_civil(year, month, day);
    (civil_from_days(days) == (year, month, day)).then_some(days)
}

/// The microseconds since midnight of a time `HH:MM:SS` with an optional fraction of up to nine
/// digits, truncated to microseconds, or `None` when it is not one.
pub(crate) fn parse_time(text: &str) -> Option<i64> {
    let (clock, fraction) = text
        .split_once('.')
        .map_or((text, ""), |(clock, f)| (clock, f));
    let mut parts = clock.split(':');
    let hour = digits(parts.next()?, 2)?;
    let minute = digits(parts.next()?, 2)?;
    let second = digits(parts.next()?, 2)?;
    if parts.next().is_some() || hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let fraction_valid = fraction.len() <= 9 && fraction.bytes().all(|byte| byte.is_ascii_digit());
    if !fraction_valid || (text.contains('.') && fraction.is_empty()) {
        return None;
    }
    let micros: String = fraction.chars().chain("000000".chars()).take(6).collect();
    Some(((hour * 60 + minute) * 60 + second) * 1_000_000 + micros.parse::<i64>().ok()?)
}

/// The microseconds since 1970-01-01T00:00:00 of a timestamp `YYYY-MM-DDTHH:MM:SS[.f]`, with a
/// final `Z` exactly when `with_time_zone`, or `None` when it is not one.
pub(crate) fn parse_timestamp(text: &str, with_time_zone: bool) -> Option<i64> {
    let text = if with_time_zone {
        text.strip_suffix('Z')?
    } else {
        text
    };
    let (date, time) = text.split_once('T')?;
    Some(parse_date(date)? * MICROSECONDS_PER_DAY + parse_time(time)?)
}

/// The date `YYYY-MM-DD` of the day `days` after 1970-01-01.
pub(crate) fn format_date(days: i64) -> String {
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// The time `HH:MM:SS`, with the fraction digits only when non-zero and without trailing zeros
/// (CON-4), of `micros` microseconds since midnight.
pub(crate) fn format_time(micros: i64) -> String {
    let seconds = micros.div_euclid(1_000_000);
    let fraction = micros.rem_euclid(1_000_000);
    let clock = format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    );
    if fraction == 0 {
        clock
    } else {
        format!("{clock}.{}", format!("{fraction:06}").trim_end_matches('0'))
    }
}

/// The CON-4 form of the timestamp `micros` microseconds after 1970-01-01T00:00:00, with a
/// final `Z` when `with_time_zone`.
pub(crate) fn format_timestamp(micros: i64, with_time_zone: bool) -> String {
    let days = micros.div_euclid(MICROSECONDS_PER_DAY);
    let time = micros.rem_euclid(MICROSECONDS_PER_DAY);
    let suffix = if with_time_zone { "Z" } else { "" };
    format!("{}T{}{suffix}", format_date(days), format_time(time))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn con_4_dates_round_trip_through_day_counts() {
        assert_eq!(parse_date("1970-01-01"), Some(0));
        assert_eq!(parse_date("2026-03-02"), Some(20_514));
        assert_eq!(parse_date("0001-01-01"), Some(-719_162));
        for days in [-719_162, -1, 0, 20_514, 2_932_896] {
            assert_eq!(parse_date(&format_date(days)), Some(days));
        }
    }

    #[test]
    fn con_4_rejects_dates_that_do_not_exist() {
        assert_eq!(parse_date("2026-02-29"), None);
        assert_eq!(parse_date("2026-13-01"), None);
        assert_eq!(parse_date("2026-1-01"), None);
    }

    #[test]
    fn con_4_times_keep_microseconds_and_drop_trailing_zeros() {
        assert_eq!(parse_time("10:30:00.5"), Some(37_800_500_000));
        assert_eq!(parse_time("10:30:00.123456789"), Some(37_800_123_456));
        assert_eq!(format_time(37_800_500_000), "10:30:00.5");
        assert_eq!(format_time(37_800_000_000), "10:30:00");
        assert_eq!(parse_time("24:00:00"), None);
        assert_eq!(parse_time("10:30:00."), None);
    }

    #[test]
    fn con_4_timestamps_with_time_zone_end_with_z() {
        let micros = parse_timestamp("2025-12-31T15:00:00Z", true).unwrap();
        assert_eq!(format_timestamp(micros, true), "2025-12-31T15:00:00Z");
        assert_eq!(parse_timestamp("2025-12-31T15:00:00", true), None);
        assert_eq!(parse_timestamp("2025-12-31T15:00:00Z", false), None);
        assert_eq!(
            format_timestamp(
                parse_timestamp("1969-12-31T23:59:59.25", false).unwrap(),
                false
            ),
            "1969-12-31T23:59:59.25"
        );
    }
}

//! Reading the text of `open_end` as a value of the logical type of `valid_to` in its CON-4
//! form (VAL-1.2, VAL-1.5).

use menura_contracts::types::common::{LogicalType, TimestampPrecision};
use menura_model::expression::DateValue;
use menura_model::internal_model::{OpenEnd, OpenEndTimestamp};

/// The value `text` writes when it is a value of `logical_type` in its CON-4 form: `YYYY-MM-DD`
/// for a `date`; `YYYY-MM-DDTHH:MM:SS`, then `.` and at most as many fraction digits as the
/// precision stores, without trailing zeros, then `Z` with time zone, for a `timestamp`.
/// `None` for any other text or type.
pub(crate) fn parse_open_end(text: &str, logical_type: &LogicalType) -> Option<OpenEnd> {
    match logical_type {
        LogicalType::Date => parse_date(text).map(OpenEnd::Date),
        LogicalType::Timestamp {
            precision,
            with_time_zone,
        } => parse_timestamp(text, *precision, *with_time_zone).map(OpenEnd::Timestamp),
        _ => None,
    }
}

/// `YYYY-MM-DD`, a day of the years 1–9999.
fn parse_date(text: &str) -> Option<DateValue> {
    let mut parts = text.split('-');
    let year = number(parts.next()?, 4)?;
    let month = number(parts.next()?, 2)?;
    let day = number(parts.next()?, 2)?;
    if parts.next().is_some() {
        return None;
    }
    DateValue::new(year, u8::try_from(month).ok()?, u8::try_from(day).ok()?)
}

/// `YYYY-MM-DDTHH:MM:SS[.fraction]`, then `Z` exactly when `with_time_zone`.
fn parse_timestamp(
    text: &str,
    precision: TimestampPrecision,
    with_time_zone: bool,
) -> Option<OpenEndTimestamp> {
    let (date, time) = text.split_once('T')?;
    let time = if with_time_zone {
        time.strip_suffix('Z')?
    } else {
        time
    };
    let (clock, fraction) = match time.split_once('.') {
        Some((clock, fraction)) => (clock, Some(fraction)),
        None => (time, None),
    };
    if !valid_clock(clock) || !fraction.is_none_or(|digits| valid_fraction(digits, precision)) {
        return None;
    }
    Some(OpenEndTimestamp {
        date: parse_date(date)?,
        time: time.to_owned(),
        with_time_zone,
    })
}

/// `HH:MM:SS` within a day.
fn valid_clock(clock: &str) -> bool {
    let mut parts = clock.split(':');
    let mut next = |length: usize, limit: u16| {
        parts
            .next()
            .and_then(|part| number(part, length))
            .is_some_and(|value| value < limit)
    };
    next(2, 24) && next(2, 60) && next(2, 60) && parts.next().is_none()
}

/// Fraction digits as CON-4 writes them, at most as many as `precision` stores: at least one,
/// the last one not zero.
fn valid_fraction(digits: &str, precision: TimestampPrecision) -> bool {
    let stored = match precision {
        TimestampPrecision::S => 0,
        TimestampPrecision::Ms => 3,
        TimestampPrecision::Us => 6,
        TimestampPrecision::Ns => 9,
    };
    !digits.is_empty()
        && digits.len() <= stored
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && !digits.ends_with('0')
}

/// The number `text` writes with exactly `length` ASCII digits.
fn number(text: &str, length: usize) -> Option<u16> {
    if text.len() != length || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stamp(precision: TimestampPrecision, with_time_zone: bool) -> LogicalType {
        LogicalType::Timestamp {
            precision,
            with_time_zone,
        }
    }

    #[test]
    fn val_1_5_a_date_reads_only_its_con4_form() {
        let date = LogicalType::Date;
        assert!(parse_open_end("9999-12-31", &date).is_some());
        for text in [
            "9999-12-32",
            "99999-12-31",
            "9999-1-31",
            "9999-12-31T00:00:00Z",
            "",
        ] {
            assert!(parse_open_end(text, &date).is_none(), "{text}");
        }
    }

    #[test]
    fn val_1_5_a_timestamp_with_time_zone_ends_with_z() {
        let instant = stamp(TimestampPrecision::Us, true);
        assert!(parse_open_end("9999-12-31T00:00:00Z", &instant).is_some());
        assert!(parse_open_end("9999-12-31T23:59:59.999999Z", &instant).is_some());
        for text in [
            "9999-12-31T00:00:00",
            "9999-12-31 00:00:00Z",
            "9999-12-31T24:00:00Z",
            "9999-12-31T00:00:00.0Z",
            "9999-12-31T00:00:00.5000Z",
            "9999-12-31T00:00:00.1234567Z",
            "9999-12-31",
        ] {
            assert!(parse_open_end(text, &instant).is_none(), "{text}");
        }
    }

    #[test]
    fn val_1_5_a_wall_clock_timestamp_has_no_z_and_fits_its_precision() {
        let seconds = stamp(TimestampPrecision::S, false);
        assert!(parse_open_end("9999-12-31T00:00:00", &seconds).is_some());
        assert!(parse_open_end("9999-12-31T00:00:00Z", &seconds).is_none());
        assert!(parse_open_end("9999-12-31T00:00:00.5", &seconds).is_none());
        let nanoseconds = stamp(TimestampPrecision::Ns, false);
        assert!(parse_open_end("2262-04-11T23:47:16.854775807", &nanoseconds).is_some());
    }

    #[test]
    fn val_1_5_no_other_type_has_an_open_end() {
        assert!(parse_open_end("1", &LogicalType::Text).is_none());
    }

    #[test]
    fn val_1_2_the_value_keeps_its_con4_text() {
        let instant = stamp(TimestampPrecision::Ms, true);
        let parsed = parse_open_end("9999-12-31T10:20:30.5Z", &instant).map(|end| end.con4_text());
        assert_eq!(parsed.as_deref(), Some("9999-12-31T10:20:30.5Z"));
    }
}

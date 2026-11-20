//! The `reference_time` of a query, read into the query tree (E1-5, QRY-2).

use menura_model::expression::{DateValue, TimeOfDay, TimestampValue};
use menura_representations::time::{ReferenceTime, UtcOffset};

/// Reads an RFC 3339 instant that satisfies the `date-time` format of the query schema.
///
/// Fraction digits beyond the microsecond are truncated toward the earlier instant, never
/// rounded: `reference_time` only selects the unit that contains it (QRY-4.2), and no unit
/// boundary falls inside a microsecond. Returns `None` for an instant that a [`ReferenceTime`]
/// cannot hold.
pub(crate) fn read_reference_time(text: &str) -> Option<ReferenceTime> {
    let (date, rest) = text.split_once(['T', 't'])?;
    let offset_start = rest.find(['Z', 'z', '+', '-'])?;
    let (time, offset) = rest.split_at(offset_start);
    let local = TimestampValue {
        date: read_date(date)?,
        time: read_time(time)?,
    };
    Some(ReferenceTime {
        local,
        offset: read_offset(offset)?,
    })
}

/// `YYYY-MM-DD`.
fn read_date(text: &str) -> Option<DateValue> {
    let mut fields = text.split('-');
    let year = fields.next()?.parse().ok()?;
    let month = fields.next()?.parse().ok()?;
    let day = fields.next()?.parse().ok()?;
    if fields.next().is_some() {
        return None;
    }
    DateValue::new(year, month, day)
}

/// `HH:MM:SS[.fraction]`, keeping at most six fraction digits.
fn read_time(text: &str) -> Option<TimeOfDay> {
    let (clock, fraction) = text.split_once('.').unwrap_or((text, ""));
    let mut fields = clock.split(':');
    let hour = fields.next()?.parse().ok()?;
    let minute = fields.next()?.parse().ok()?;
    let second = fields.next()?.parse().ok()?;
    if fields.next().is_some() {
        return None;
    }
    let kept = fraction
        .get(..TimeOfDay::MAXIMUM_FRACTION_DIGITS)
        .unwrap_or(fraction);
    TimeOfDay::new(hour, minute, second, kept)
}

/// `Z` or `±HH:MM`.
fn read_offset(text: &str) -> Option<UtcOffset> {
    if text.eq_ignore_ascii_case("z") {
        return UtcOffset::from_minutes(0);
    }
    let (sign, numeric) = match text.split_at_checked(1)? {
        ("+", numeric) => (1, numeric),
        ("-", numeric) => (-1, numeric),
        _ => return None,
    };
    let (hours, minutes) = numeric.split_once(':')?;
    let hours: i16 = hours.parse().ok()?;
    let minutes: i16 = minutes.parse().ok()?;
    UtcOffset::from_minutes(sign * (hours * 60 + minutes))
}

#[cfg(test)]
mod tests {
    use super::read_reference_time;

    #[test]
    fn e1_5_truncates_nine_fraction_digits_to_six() {
        let time = read_reference_time("2026-12-31T23:59:59.999999999Z").expect("an instant");
        assert_eq!(time.local.time.fraction_digits(), "999999");
        assert_eq!(time.local.time.second(), 59);
        assert_eq!(time.local.date.day(), 31);
    }

    #[test]
    fn e1_5_keeps_short_fractions_as_written() {
        let time = read_reference_time("2026-12-31T23:59:59.50Z").expect("an instant");
        assert_eq!(time.local.time.fraction_digits(), "50");
    }

    #[test]
    fn e1_5_reads_the_offset_sign() {
        let time = read_reference_time("2026-01-01T00:00:00-23:59").expect("an instant");
        assert_eq!(time.offset.minutes(), -(23 * 60 + 59));
    }
}

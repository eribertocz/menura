//! The reference time of a query as an instant in UTC, encoded as CON-4 requires for a
//! `timestamp with time zone` parameter.

use menura_model::expression::{DateValue, TimeOfDay, TimestampValue};
use menura_representations::time::ReferenceTime;

use crate::values::timestamp_text;

const MINUTES_PER_DAY: i64 = 24 * 60;

/// `reference` converted to UTC, as `YYYY-MM-DDTHH:MM:SS[.fraction]Z`, or `None` when the UTC
/// date falls outside the years 0001–9999 (E1 rejects such a reference time, QRY-2).
pub(crate) fn utc_text(reference: &ReferenceTime) -> Option<String> {
    let local = &reference.local;
    let minutes = i64::from(local.time.hour()) * 60 + i64::from(local.time.minute())
        - i64::from(reference.offset.minutes());
    let day_shift = minutes.div_euclid(MINUTES_PER_DAY);
    let minute_of_day = minutes.rem_euclid(MINUTES_PER_DAY);
    let date = date_from_days(days_from_date(local.date) + day_shift)?;
    let time = TimeOfDay::new(
        u8::try_from(minute_of_day / 60).ok()?,
        u8::try_from(minute_of_day % 60).ok()?,
        local.time.second(),
        local.time.fraction_digits(),
    )?;
    Some(format!(
        "{}Z",
        timestamp_text(&TimestampValue { date, time })
    ))
}

/// Days since 1970-01-01 of `date`, in the proleptic Gregorian calendar.
fn days_from_date(date: DateValue) -> i64 {
    let month = i64::from(date.month());
    let year = i64::from(date.year()) - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + i64::from(date.day()) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The date `days` after 1970-01-01, or `None` outside the years 0001–9999.
fn date_from_days(days: i64) -> Option<DateValue> {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
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
    DateValue::new(
        u16::try_from(year).ok()?,
        u8::try_from(month).ok()?,
        u8::try_from(day).ok()?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use menura_representations::time::UtcOffset;

    fn reference(date: (u16, u8, u8), time: (u8, u8), offset: i16) -> ReferenceTime {
        ReferenceTime {
            local: TimestampValue {
                date: DateValue::new(date.0, date.1, date.2).unwrap(),
                time: TimeOfDay::new(time.0, time.1, 5, "25").unwrap(),
            },
            offset: UtcOffset::from_minutes(offset).unwrap(),
        }
    }

    #[test]
    fn a_west_offset_can_move_to_the_next_day() {
        let text = utc_text(&reference((2026, 12, 31), (23, 30), -240));
        assert_eq!(text.as_deref(), Some("2027-01-01T03:30:05.25Z"));
    }

    #[test]
    fn an_east_offset_can_move_to_the_previous_day() {
        let text = utc_text(&reference((2024, 3, 1), (0, 15), 330));
        assert_eq!(text.as_deref(), Some("2024-02-29T18:45:05.25Z"));
    }

    #[test]
    fn a_date_before_year_one_is_none() {
        assert_eq!(utc_text(&reference((1, 1, 1), (0, 0), 60)), None);
    }
}

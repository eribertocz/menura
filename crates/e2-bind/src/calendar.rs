//! Calendar arithmetic over dates of the proleptic Gregorian calendar, for the whole units of
//! QRY-4.2: day, ISO week (starting Monday), month, quarter and year.

use menura_contracts::types::common::Grain;
use menura_model::expression::DateValue;

/// The `count` whole units ending with the unit that contains `date` (when `include_current`)
/// or with the unit before it, as the first and the last date, both inclusive. Returns `None`
/// when a bound falls outside the years 1–9999.
pub(crate) fn whole_units(
    date: DateValue,
    unit: Grain,
    count: u64,
    include_current: bool,
) -> Option<(DateValue, DateValue)> {
    let count = i64::try_from(count).ok()?;
    let current = unit_index(date, unit);
    let last = if include_current {
        current
    } else {
        current.checked_sub(1)?
    };
    let first = last.checked_sub(count.checked_sub(1)?)?;
    let from = day_date(unit_start(first, unit)?)?;
    let to = day_date(unit_start(last.checked_add(1)?, unit)?.checked_sub(1)?)?;
    Some((from, to))
}

/// The index of the unit that contains `date`: consecutive units have consecutive indexes.
fn unit_index(date: DateValue, unit: Grain) -> i64 {
    let months = i64::from(date.year()) * 12 + i64::from(date.month()) - 1;
    match unit {
        Grain::Day => day_number(date),
        // Day 0 is 1970-01-01, a Thursday; day 4 is the Monday 1970-01-05.
        Grain::Week => (day_number(date) - 4).div_euclid(7),
        Grain::Month => months,
        Grain::Quarter => months.div_euclid(3),
        Grain::Year => i64::from(date.year()),
    }
}

/// The day number of the first date of the unit of `index`.
fn unit_start(index: i64, unit: Grain) -> Option<i64> {
    match unit {
        Grain::Day => Some(index),
        Grain::Week => index.checked_mul(7)?.checked_add(4),
        Grain::Month => month_start(index),
        Grain::Quarter => month_start(index.checked_mul(3)?),
        Grain::Year => month_start(index.checked_mul(12)?),
    }
}

/// The day number of the first day of the month of index `year * 12 + month - 1`.
fn month_start(months: i64) -> Option<i64> {
    let year = months.div_euclid(12);
    let month = months.rem_euclid(12) + 1;
    (year.abs() < 1_000_000).then(|| civil_day_number(year, month, 1))
}

/// The number of days from 1970-01-01 to `date`.
fn day_number(date: DateValue) -> i64 {
    civil_day_number(
        i64::from(date.year()),
        i64::from(date.month()),
        i64::from(date.day()),
    )
}

/// The number of days from 1970-01-01 to the date `year-month-day`.
fn civil_day_number(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let shifted_month = (month + 9) % 12;
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}
/// The date `days` after 1970-01-01, when its year is 1–9999.
fn day_date(days: i64) -> Option<DateValue> {
    let shifted = days.checked_add(719_468)?;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    DateValue::new(
        u16::try_from(year).ok()?,
        u8::try_from(month).ok()?,
        u8::try_from(day).ok()?,
    )
}

/// The date written `YYYY-MM-DD`, when it is one.
pub(crate) fn parse_date(text: &str) -> Option<DateValue> {
    let mut parts = text.split('-');
    let year = parts.next()?;
    let month = parts.next()?;
    let day = parts.next()?;
    let digits = |part: &str, length: usize| {
        part.len() == length && part.bytes().all(|byte| byte.is_ascii_digit())
    };
    if parts.next().is_some() || !digits(year, 4) || !digits(month, 2) || !digits(day, 2) {
        return None;
    }
    DateValue::new(year.parse().ok()?, month.parse().ok()?, day.parse().ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(text: &str) -> DateValue {
        parse_date(text).expect("a date")
    }

    #[test]
    fn day_numbers_round_trip() {
        for text in [
            "0001-01-01",
            "1970-01-01",
            "2000-02-29",
            "2026-11-20",
            "9999-12-31",
        ] {
            assert_eq!(day_date(day_number(date(text))), Some(date(text)));
        }
        assert_eq!(day_number(date("1970-01-01")), 0);
    }

    #[test]
    fn a_week_starts_on_monday() {
        // 2026-11-20 is a Friday.
        assert_eq!(
            whole_units(date("2026-11-20"), Grain::Week, 1, true),
            Some((date("2026-11-16"), date("2026-11-22")))
        );
    }

    #[test]
    fn bounds_outside_the_years_1_to_9999_are_none() {
        assert_eq!(whole_units(date("0001-01-01"), Grain::Day, 1, false), None);
        assert_eq!(
            whole_units(date("2026-01-01"), Grain::Year, u64::MAX, true),
            None
        );
        assert_eq!(
            whole_units(date("9999-12-31"), Grain::Year, 1, true),
            Some((date("9999-01-01"), date("9999-12-31")))
        );
    }
}

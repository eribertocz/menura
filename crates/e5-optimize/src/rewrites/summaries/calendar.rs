//! The periods of a time grain: whether a date starts or ends one (SUM-3.5), and whether one
//! grain derives from another by exact grouping (SUM-3.4).

use menura_contracts::types::common::Grain;
use menura_model::expression::{DateUnit, DateValue};

/// Whether the query unit `unit` derives from the summary grain `grain` by exact grouping
/// (SUM-3.4): `day` → all; `week` → `week`; `month` → `month`, `quarter`, `year`; `quarter`
/// → `quarter`, `year`; `year` → `year`.
pub(super) fn derives(grain: Grain, unit: DateUnit) -> bool {
    match grain {
        Grain::Day => matches!(
            unit,
            DateUnit::Day | DateUnit::Week | DateUnit::Month | DateUnit::Quarter | DateUnit::Year
        ),
        Grain::Week => unit == DateUnit::Week,
        Grain::Month => matches!(unit, DateUnit::Month | DateUnit::Quarter | DateUnit::Year),
        Grain::Quarter => matches!(unit, DateUnit::Quarter | DateUnit::Year),
        Grain::Year => unit == DateUnit::Year,
    }
}

/// Whether the date range from `from` to `to` is aligned on `grain` (SUM-3.5): `from` is the
/// first day of a period and `to` the last day of one, or `9999-12-31`.
pub(super) fn is_aligned(from: DateValue, to: DateValue, grain: Grain) -> bool {
    let open_ended = (to.year(), to.month(), to.day()) == (9999, 12, 31);
    starts_period(from, grain) && (open_ended || ends_period(to, grain))
}

fn starts_period(date: DateValue, grain: Grain) -> bool {
    match grain {
        Grain::Day => true,
        Grain::Week => weekday(date) == 0,
        Grain::Month => date.day() == 1,
        Grain::Quarter => date.day() == 1 && date.month() % 3 == 1,
        Grain::Year => date.day() == 1 && date.month() == 1,
    }
}

fn ends_period(date: DateValue, grain: Grain) -> bool {
    let last_of_month = date.day() == days_in_month(date.year(), date.month());
    match grain {
        Grain::Day => true,
        Grain::Week => weekday(date) == 6,
        Grain::Month => last_of_month,
        Grain::Quarter => last_of_month && date.month().is_multiple_of(3),
        Grain::Year => last_of_month && date.month() == 12,
    }
}

fn is_leap_year(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// The ISO day of the week of `date`, from 0 for Monday to 6 for Sunday.
fn weekday(date: DateValue) -> u32 {
    // Days since 0001-01-01, a Monday in the proleptic Gregorian calendar.
    let year = u32::from(date.year()) - 1;
    let mut days = year * 365 + year / 4 - year / 100 + year / 400;
    for month in 1..date.month() {
        days += u32::from(days_in_month(date.year(), month));
    }
    days += u32::from(date.day()) - 1;
    days % 7
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: u16, month: u8, day: u8) -> DateValue {
        DateValue::new(year, month, day).expect("a date")
    }

    #[test]
    fn sum_3_5_weeks_start_on_monday() {
        assert!(is_aligned(
            date(2026, 11, 16),
            date(2026, 11, 22),
            Grain::Week
        ));
        assert!(!is_aligned(
            date(2026, 11, 15),
            date(2026, 11, 22),
            Grain::Week
        ));
    }

    #[test]
    fn sum_3_5_periods_end_on_their_last_day() {
        assert!(is_aligned(
            date(2024, 1, 1),
            date(2024, 2, 29),
            Grain::Month
        ));
        assert!(!is_aligned(
            date(2023, 1, 1),
            date(2023, 2, 28),
            Grain::Quarter
        ));
        assert!(is_aligned(
            date(2023, 4, 1),
            date(2023, 6, 30),
            Grain::Quarter
        ));
        assert!(is_aligned(
            date(2023, 1, 1),
            date(9999, 12, 31),
            Grain::Year
        ));
    }
}

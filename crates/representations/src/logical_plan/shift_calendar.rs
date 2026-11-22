//! The shift of [`PeriodShift`] on calendar dates, for the decisions that read the shifted
//! range at compile time, such as the coverage of a summary (E4-8.9). The plan never binds
//! these dates: the SQL computes the same days from the query's own dates (E6-4.14).

use menura_contracts::types::common::Grain;
use menura_model::expression::DateValue;

use super::shift::PeriodShift;

/// The start of the period of `grain` that contains `date`; weeks start on Monday (ISO).
pub(super) fn start_of(date: DateValue, grain: Grain) -> Option<DateValue> {
    let (year, month) = (date.year(), date.month());
    match grain {
        Grain::Day => Some(date),
        Grain::Week => from_days(days(date) - i64::from(weekday(date))),
        Grain::Month => DateValue::new(year, month, 1),
        Grain::Quarter => DateValue::new(year, (month - 1) / 3 * 3 + 1, 1),
        Grain::Year => DateValue::new(year, 1, 1),
    }
}

/// The start of the period of `grain` after the one that starts at `start`.
pub(super) fn start_of_next(start: DateValue, grain: Grain) -> Option<DateValue> {
    match grain {
        Grain::Day => from_days(days(start) + 1),
        Grain::Week => from_days(days(start) + 7),
        Grain::Month => add_months(start, 1),
        Grain::Quarter => add_months(start, 3),
        Grain::Year => add_months(start, 12),
    }
}

/// The start `start` of a query-grain period shifted back as [`PeriodShift::shifted`] states.
pub(super) fn shifted_start_date(start: DateValue, shift: PeriodShift) -> Option<DateValue> {
    let offset = i64::try_from(shift.offset.get()).ok()?;
    match (shift.query_grain, shift.grain) {
        (Grain::Week, Grain::Month | Grain::Quarter) => {
            start_of(back(start, shift.grain, offset)?, Grain::Week)
        }
        (Grain::Week, Grain::Year) => week_in_shifted_year(start, offset),
        _ => back(start, shift.grain, offset),
    }
}

/// `date` moved back `offset` units of `grain`, the day clamped to the target month.
fn back(date: DateValue, grain: Grain, offset: i64) -> Option<DateValue> {
    match grain {
        Grain::Day => from_days(days(date) - offset),
        Grain::Week => from_days(days(date) - offset.checked_mul(7)?),
        Grain::Month => add_months(date, -offset),
        Grain::Quarter => add_months(date, offset.checked_mul(-3)?),
        Grain::Year => add_months(date, offset.checked_mul(-12)?),
    }
}

/// The Monday of the week with the number of the week of `monday` in the ISO year `offset`
/// years earlier, or of its last week when it has fewer.
fn week_in_shifted_year(monday: DateValue, offset: i64) -> Option<DateValue> {
    let thursday = from_days(days(monday) + 3)?;
    let week = (day_of_year(thursday) - 1) / 7 + 1;
    let year = i64::from(thursday.year()) - offset;
    let year = u16::try_from(year).ok()?;
    let fourth = DateValue::new(year, 1, 4)?;
    let first_monday = days(fourth) - i64::from(weekday(fourth));
    let last_week = if has_week_53(year) { 53 } else { 52 };
    from_days(first_monday + 7 * (week.min(last_week) - 1))
}

/// Whether the ISO year `year` has 53 weeks: it starts on a Thursday, or it is a leap year
/// that starts on a Wednesday.
fn has_week_53(year: u16) -> bool {
    let Some(first) = DateValue::new(year, 1, 1) else {
        return false;
    };
    let starts = weekday(first);
    starts == 3 || (starts == 2 && DateValue::new(year, 2, 29).is_some())
}

/// `date` plus `months` months, the day clamped to the last day of the target month.
fn add_months(date: DateValue, months: i64) -> Option<DateValue> {
    let index = i64::from(date.year()) * 12 + i64::from(date.month()) - 1 + months;
    let year = u16::try_from(index.div_euclid(12)).ok()?;
    let month = u8::try_from(index.rem_euclid(12) + 1).ok()?;
    (1..=date.day())
        .rev()
        .find_map(|day| DateValue::new(year, month, day))
}

/// The day of the week of `date`, from 0 for Monday to 6 for Sunday.
fn weekday(date: DateValue) -> u8 {
    // 1970-01-01 is a Thursday, day 3.
    u8::try_from((days(date) + 3).rem_euclid(7)).unwrap_or(0)
}

/// The day of the year of `date`, from 1.
fn day_of_year(date: DateValue) -> i64 {
    DateValue::new(date.year(), 1, 1).map_or(1, |first| days(date) - days(first) + 1)
}

/// The number of days from 1970-01-01 to `date` (proleptic Gregorian calendar).
fn days(date: DateValue) -> i64 {
    let month = i64::from(date.month());
    let year = i64::from(date.year()) - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + i64::from(date.day()) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The date `count` days after 1970-01-01; `None` outside the years 0001–9999.
pub(super) fn from_days(count: i64) -> Option<DateValue> {
    let shifted = count + 719_468;
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
    DateValue::new(
        u16::try_from(year).ok()?,
        u8::try_from(month).ok()?,
        u8::try_from(day).ok()?,
    )
}

/// The day before `date`.
pub(super) fn previous_day(date: DateValue) -> Option<DateValue> {
    from_days(days(date) - 1)
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU64;

    use super::*;

    fn date(year: u16, month: u8, day: u8) -> DateValue {
        DateValue::new(year, month, day).expect("a valid date")
    }

    fn shift(query_grain: Grain, grain: Grain) -> PeriodShift {
        PeriodShift {
            query_grain,
            grain,
            offset: NonZeroU64::MIN,
        }
    }

    #[test]
    fn ctx_3a_2_a_day_shifted_by_a_month_takes_the_last_day_of_the_target_month() {
        let shifted = shifted_start_date(date(2024, 3, 31), shift(Grain::Day, Grain::Month));
        assert_eq!(shifted, Some(date(2024, 2, 29)));
    }

    #[test]
    fn ctx_3a_2_29_february_shifted_by_a_year_is_28_february() {
        let shifted = shifted_start_date(date(2024, 2, 29), shift(Grain::Day, Grain::Year));
        assert_eq!(shifted, Some(date(2023, 2, 28)));
    }

    #[test]
    fn ctx_3a_2_week_53_shifted_by_a_year_is_week_52() {
        let shifted = shifted_start_date(date(2020, 12, 28), shift(Grain::Week, Grain::Year));
        assert_eq!(shifted, Some(date(2019, 12, 23)));
    }

    #[test]
    fn ctx_3a_2_a_week_shifted_by_a_quarter_is_the_week_of_its_shifted_monday() {
        let shifted = shifted_start_date(date(2024, 2, 26), shift(Grain::Week, Grain::Quarter));
        assert_eq!(shifted, Some(date(2023, 11, 20)));
    }

    #[test]
    fn e4_8_4_the_shifted_range_runs_over_whole_shifted_periods() {
        let dates =
            shift(Grain::Month, Grain::Year).shifted_dates(date(2024, 2, 10), date(2024, 3, 31));
        assert_eq!(dates, Some((date(2023, 2, 1), date(2023, 3, 31))));
    }
}

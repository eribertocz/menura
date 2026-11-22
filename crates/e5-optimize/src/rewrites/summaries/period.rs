//! The periods of the catalog state (CAT-5.1, CAT-8.6): their grain, their days, and the period
//! of a grain that contains a day.

use menura_contracts::types::common::Grain;
use menura_model::expression::DateValue;

/// A calendar day, counted from 1970-01-01 in the proleptic Gregorian calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Day(i32);

impl Day {
    /// The day of `date`.
    pub(super) fn of_date(date: DateValue) -> Self {
        Self::from_civil(
            i32::from(date.year()),
            i32::from(date.month()),
            i32::from(date.day()),
        )
    }

    /// The day `day` of month `month` of year `year`, for a month 1–12 and a day 1–31.
    fn from_civil(year: i32, month: i32, day: i32) -> Self {
        let year = if month <= 2 { year - 1 } else { year };
        let era = year.div_euclid(400);
        let year_of_era = year - era * 400;
        let shifted_month = (month + 9) % 12;
        let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        Self(era * 146_097 + day_of_era - 719_468)
    }

    /// The year, month and day of this day.
    fn to_civil(self) -> (i32, i32, i32) {
        let shifted = self.0 + 719_468;
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
        let year = year_of_era + era * 400 + i32::from(month <= 2);
        (year, month, day)
    }

    /// The day `count` days after this one.
    fn plus(self, count: i32) -> Self {
        Self(self.0 + count)
    }

    /// The Monday of the ISO week of this day.
    fn monday(self) -> Self {
        // 1970-01-01 is a Thursday, the fourth day of its week counted from Monday.
        Self(self.0 - (self.0 + 3).rem_euclid(7))
    }
}

/// A period of the catalog state: its grain and its first day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Period {
    grain: Grain,
    first: Day,
}

impl Period {
    /// The period that `text` denotes by its shape (CAT-8.6): `YYYY` year, `YYYY-Qn` quarter,
    /// `YYYY-MM` month, `YYYY-Www` ISO week, `YYYY-MM-DD` day. `None` when `text` has none of
    /// these shapes or names no real calendar date or ISO week of the years 0001–9999.
    pub(super) fn parse(text: &str) -> Option<Self> {
        let year = digits(text.get(0..4)?, 4).filter(|year| *year >= 1)?;
        let rest = text.get(4..)?;
        if rest.is_empty() {
            return Some(Self::containing(Day::from_civil(year, 1, 1), Grain::Year));
        }
        let rest = rest.strip_prefix('-')?;
        if let Some(quarter) = rest.strip_prefix('Q') {
            let quarter = digits(quarter, 1).filter(|quarter| (1..=4).contains(quarter))?;
            let first = Day::from_civil(year, quarter * 3 - 2, 1);
            return Some(Self::containing(first, Grain::Quarter));
        }
        if let Some(week) = rest.strip_prefix('W') {
            return iso_week(year, week);
        }
        let month = digits(rest.get(0..2)?, 2).filter(|month| (1..=12).contains(month))?;
        let rest = rest.get(2..)?;
        if rest.is_empty() {
            return Some(Self::containing(
                Day::from_civil(year, month, 1),
                Grain::Month,
            ));
        }
        let day = digits(rest.strip_prefix('-')?, 2)?;
        let date = Day::from_civil(year, month, day);
        (day >= 1 && date.to_civil() == (year, month, day))
            .then(|| Self::containing(date, Grain::Day))
    }

    /// The period of `grain` that contains `day`.
    pub(super) fn containing(day: Day, grain: Grain) -> Self {
        let (year, month, _) = day.to_civil();
        let first = match grain {
            Grain::Day => day,
            Grain::Week => day.monday(),
            Grain::Month => Day::from_civil(year, month, 1),
            Grain::Quarter => Day::from_civil(year, (month - 1) / 3 * 3 + 1, 1),
            Grain::Year => Day::from_civil(year, 1, 1),
        };
        Self { grain, first }
    }

    /// The grain of the period.
    pub(super) fn grain(self) -> Grain {
        self.grain
    }

    /// The first day of the period.
    pub(super) fn first_day(self) -> Day {
        self.first
    }

    /// The last day of the period.
    pub(super) fn last_day(self) -> Day {
        let (year, month, _) = self.first.to_civil();
        let next = match self.grain {
            Grain::Day => self.first.plus(1),
            Grain::Week => self.first.plus(7),
            Grain::Month => first_of_month_after(year, month, 1),
            Grain::Quarter => first_of_month_after(year, month, 3),
            Grain::Year => Day::from_civil(year + 1, 1, 1),
        };
        next.plus(-1)
    }
}

/// The first day of the month `count` months after month `month` of year `year`.
fn first_of_month_after(year: i32, month: i32, count: i32) -> Day {
    let index = year * 12 + (month - 1) + count;
    Day::from_civil(index.div_euclid(12), index.rem_euclid(12) + 1, 1)
}

/// The ISO week `week` (two digits) of the ISO week-numbering year `year`. Week 1 is the week
/// that contains 4 January.
fn iso_week(year: i32, week: &str) -> Option<Period> {
    let week = digits(week, 2).filter(|week| (1..=53).contains(week))?;
    let first = Day::from_civil(year, 1, 4).monday().plus((week - 1) * 7);
    // A week belongs to the ISO year of its Thursday; week 53 exists only in some years.
    let (thursday_year, _, _) = first.plus(3).to_civil();
    (thursday_year == year).then_some(Period {
        grain: Grain::Week,
        first,
    })
}

/// The value of `text` when it is exactly `length` ASCII digits.
fn digits(text: &str, length: usize) -> Option<i32> {
    let well_formed = text.len() == length && text.bytes().all(|byte| byte.is_ascii_digit());
    well_formed.then(|| {
        text.bytes()
            .fold(0, |value, byte| value * 10 + i32::from(byte - b'0'))
    })
}

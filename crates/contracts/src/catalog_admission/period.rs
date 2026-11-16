//! Canonical periods (CAT-5.1).

use crate::types::common::Grain;

/// The grain whose canonical form `period` has, or `None` when it is in no canonical form.
///
/// A canonical period names a real period of the proleptic Gregorian calendar: `2026-02-29`
/// and `2025-W53` are in no canonical form. Two canonical periods of one grain compare in
/// chronological order as text.
pub(super) fn canonical_grain(period: &str) -> Option<Grain> {
    let year = digits(period.get(..4)?)?;
    let rest = period.get(4..)?;
    if rest.is_empty() {
        return Some(Grain::Year);
    }
    let rest = rest.strip_prefix('-')?;
    if let Some(quarter) = rest.strip_prefix('Q') {
        return matches!(quarter, "1" | "2" | "3" | "4").then_some(Grain::Quarter);
    }
    if let Some(week) = rest.strip_prefix('W') {
        let week = two_digits(week)?;
        return (1..=iso_weeks_in_year(year))
            .contains(&week)
            .then_some(Grain::Week);
    }
    let month = two_digits(rest.get(..2)?)?;
    if !(1..=12).contains(&month) {
        return None;
    }
    let day = rest.get(2..)?;
    if day.is_empty() {
        return Some(Grain::Month);
    }
    let day = two_digits(day.strip_prefix('-')?)?;
    (1..=days_in_month(year, month))
        .contains(&day)
        .then_some(Grain::Day)
}

/// The value of exactly four ASCII digits.
fn digits(text: &str) -> Option<i64> {
    fixed_digits(text, 4)
}

/// The value of exactly two ASCII digits.
fn two_digits(text: &str) -> Option<i64> {
    fixed_digits(text, 2)
}

fn fixed_digits(text: &str, length: usize) -> Option<i64> {
    if text.len() != length || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// The number of ISO 8601 weeks of a week-numbering year: 53 when the year ends on a Thursday,
/// or the previous year ends on a Wednesday; 52 otherwise.
fn iso_weeks_in_year(year: i64) -> i64 {
    if weekday_of_december_31(year) == 4 || weekday_of_december_31(year - 1) == 3 {
        53
    } else {
        52
    }
}

/// The weekday of 31 December of `year`, from 0 (Sunday) to 6 (Saturday).
fn weekday_of_december_31(year: i64) -> i64 {
    (year + year.div_euclid(4) - year.div_euclid(100) + year.div_euclid(400)).rem_euclid(7)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cat_5_1_every_grain_has_one_canonical_form() {
        let cases = [
            ("2026-10-31", Some(Grain::Day)),
            ("2024-02-29", Some(Grain::Day)),
            ("2026-W44", Some(Grain::Week)),
            ("2026-10", Some(Grain::Month)),
            ("2026-Q4", Some(Grain::Quarter)),
            ("2026", Some(Grain::Year)),
            ("2026-02-29", None),
            ("1900-02-29", None),
            ("2026-04-31", None),
            ("2026-13", None),
            ("2026-W00", None),
            ("2026-Q5", None),
            ("2026-1", None),
            ("26", None),
            ("2026-10-31T00", None),
        ];
        for (period, grain) in cases {
            assert_eq!(canonical_grain(period), grain, "{period}");
        }
    }

    #[test]
    fn cat_5_1_iso_years_with_53_weeks() {
        let long_years: Vec<i64> = (2000..=2030)
            .filter(|year| iso_weeks_in_year(*year) == 53)
            .collect();
        assert_eq!(long_years, [2004, 2009, 2015, 2020, 2026]);
    }
}

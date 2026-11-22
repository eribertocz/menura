//! Unit tests of the canonical periods (CAT-5.1, CAT-8.6).

use menura_contracts::types::common::Grain;
use menura_model::expression::DateValue;

use super::period::{Day, Period};

/// The day `year-month-day`.
fn day(year: u16, month: u8, day: u8) -> Day {
    Day::of_date(DateValue::new(year, month, day).expect("a date"))
}

/// The period `text`, which must be canonical.
fn period(text: &str) -> Period {
    Period::parse(text).expect("a canonical period")
}

#[test]
fn cat_5_1_each_shape_denotes_its_grain() {
    assert_eq!(period("2026").grain(), Grain::Year);
    assert_eq!(period("2026-Q4").grain(), Grain::Quarter);
    assert_eq!(period("2026-10").grain(), Grain::Month);
    assert_eq!(period("2026-W44").grain(), Grain::Week);
    assert_eq!(period("2026-10-31").grain(), Grain::Day);
}

#[test]
fn cat_5_1_iso_weeks_belong_to_the_year_of_their_thursday() {
    let week = period("2020-W53");
    assert_eq!(week.first_day(), day(2020, 12, 28));
    assert_eq!(week.last_day(), day(2021, 1, 3));
    assert_eq!(Period::containing(day(2021, 1, 3), Grain::Week), week);
    assert_eq!(period("2021-W01").first_day(), day(2021, 1, 4));
}

#[test]
fn cat_5_1_periods_span_their_days() {
    assert_eq!(period("2024-02").last_day(), day(2024, 2, 29));
    assert_eq!(period("2026-Q4").first_day(), day(2026, 10, 1));
    assert_eq!(period("2026-Q4").last_day(), day(2026, 12, 31));
    assert_eq!(period("2026").last_day(), day(2026, 12, 31));
    let quarter = Period::containing(day(2026, 11, 15), Grain::Quarter);
    assert_eq!(quarter, period("2026-Q4"));
}

#[test]
fn cat_8_6_rejects_periods_that_name_no_real_date_or_week() {
    for text in [
        "2026-02-29",
        "2025-W53",
        "2026-W00",
        "2026-13",
        "2026-Q5",
        "0000",
        "2026-W4",
        "2026-1-01",
        "26-10",
        "2026-10-31x",
        "2026-00-10",
    ] {
        assert_eq!(Period::parse(text), None, "{text}");
    }
    assert!(Period::parse("2026-W53").is_some());
    assert!(Period::parse("2024-02-29").is_some());
}

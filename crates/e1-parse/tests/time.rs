//! E1-5 "Time zone, reference time": the query time of the tree.

// `cfg(test)` marks the helpers as test code, where `expect` and `panic` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_e1_parse::ParseErrorCode;
use menura_representations::time::ReferenceTime;
use serde_json::{Value, json};
use support::{accepted, assert_rejected, rejected, wide_table_query};

#[cfg(test)]
fn with_reference_time(text: &str) -> Value {
    let mut document = wide_table_query();
    document["reference_time"] = json!(text);
    document
}

#[cfg(test)]
fn reference_time(text: &str) -> ReferenceTime {
    let time = accepted(&with_reference_time(text)).tree.time().clone();
    time.reference_time.expect("a reference time")
}

#[test]
fn e1_5_keeps_the_time_zone_as_written() {
    let tree = accepted(&wide_table_query()).tree;
    assert_eq!(tree.time().time_zone.as_str(), "Europe/Madrid");
    assert_eq!(tree.time().reference_time, None);
}

#[test]
fn e1_5_reads_the_reference_time_with_its_offset() {
    let time = reference_time("2026-03-31T10:15:30.25+02:00");
    let local = &time.local;
    assert_eq!(
        (local.date.year(), local.date.month(), local.date.day()),
        (2026, 3, 31)
    );
    let clock = (local.time.hour(), local.time.minute(), local.time.second());
    assert_eq!(clock, (10, 15, 30));
    assert_eq!(local.time.fraction_digits(), "25");
    assert_eq!(time.offset.minutes(), 120);
}

#[test]
fn e1_5_reads_utc_and_negative_offsets() {
    assert_eq!(reference_time("2026-03-31T10:00:00Z").offset.minutes(), 0);
    assert_eq!(reference_time("2026-03-31t10:00:00z").offset.minutes(), 0);
    assert_eq!(
        reference_time("2026-03-31T10:00:00-05:30").offset.minutes(),
        -330
    );
}

#[test]
fn e1_5_truncates_fraction_digits_beyond_microseconds() {
    let time = reference_time("2026-12-31T23:59:59.999999999+01:00");
    let local = &time.local;
    assert_eq!(
        (local.date.year(), local.date.month(), local.date.day()),
        (2026, 12, 31)
    );
    let clock = (local.time.hour(), local.time.minute(), local.time.second());
    assert_eq!(clock, (23, 59, 59));
    assert_eq!(local.time.fraction_digits(), "999999");
    assert_eq!(time.offset.minutes(), 60);
}

#[test]
fn qry_2_reads_a_leap_second_as_the_last_microsecond_of_the_minute() {
    for text in ["2026-12-31T23:59:60+01:00", "2026-12-31T23:59:60.5+01:00"] {
        let time = reference_time(text);
        let local = &time.local;
        assert_eq!((local.date.month(), local.date.day()), (12, 31));
        let clock = (local.time.hour(), local.time.minute(), local.time.second());
        assert_eq!(clock, (23, 59, 59), "{text}");
        assert_eq!(local.time.fraction_digits(), "999999", "{text}");
    }
}

#[test]
fn qry_2_year_zero_is_invalid_query_at_reference_time() {
    let document = with_reference_time("0000-06-01T00:00:00Z");
    assert_rejected(&document, ParseErrorCode::InvalidQuery, "/reference_time");
}

#[test]
fn qry_2_year_outside_the_range_after_conversion_to_utc_is_invalid_query() {
    for text in ["0001-01-01T00:30:00+01:00", "9999-12-31T23:30:00-01:00"] {
        let document = with_reference_time(text);
        assert_rejected(&document, ParseErrorCode::InvalidQuery, "/reference_time");
    }
}

#[test]
fn qry_2_the_first_and_last_instants_of_the_range_are_accepted() {
    assert_eq!(
        reference_time("0001-01-01T01:00:00+01:00").offset.minutes(),
        60
    );
    assert_eq!(
        reference_time("9999-12-31T22:59:59.999999-01:00")
            .offset
            .minutes(),
        -60
    );
}

#[test]
fn qry_2_reference_time_range_is_checked_with_the_json_schema() {
    let mut document = with_reference_time("0000-06-01T00:00:00Z");
    document["query"]["filter"] = json!("orders.amount >");
    assert_eq!(rejected(&document).code, ParseErrorCode::InvalidQuery);

    let mut sql = with_reference_time("0000-06-01T00:00:00Z");
    sql.as_object_mut().map(|members| members.remove("query"));
    sql["sql"] = json!("SELECT 1");
    assert_rejected(&sql, ParseErrorCode::InvalidQuery, "/reference_time");
}

#[test]
fn qry_4_2_the_reference_date_is_the_date_of_the_reference_time_in_the_time_zone() {
    // 23:30 UTC on 2026-03-31 is already 2026-04-01 in Madrid (UTC+2).
    let tree = accepted(&with_reference_time("2026-03-31T23:30:00Z")).tree;
    let date = tree.time().reference_date.expect("a reference date");
    assert_eq!((date.year(), date.month(), date.day()), (2026, 4, 1));
}

#[test]
fn qry_4_2_there_is_no_reference_date_without_a_reference_time() {
    let tree = accepted(&wide_table_query()).tree;
    assert_eq!(tree.time().reference_date, None);
}

//! The parameter bindings of the reference executor on `ClickHouse` (EXE-2): each row of the
//! EXE-2.3 table that has a `ClickHouse` value, written as the query parameter `pn`.

use menura_contracts::types::common::{
    FloatBits, IntegerBits, LogicalType, Parameter, TimestampPrecision,
};
use serde_json::{Value as Json, json};

use super::parameter_texts;

/// The text bound to `p1` for the CON-4 value `value` of `logical_type`.
fn text(logical_type: LogicalType, value: Json) -> Result<String, String> {
    let parameters = [Parameter {
        position: 1,
        logical_type,
        value,
    }];
    parameter_texts(&parameters).map(|mut texts| texts.remove(0).1)
}

fn integer(bits: IntegerBits, signed: bool) -> LogicalType {
    LogicalType::Integer { bits, signed }
}

fn timestamp(with_time_zone: bool) -> LogicalType {
    LogicalType::Timestamp {
        precision: TimestampPrecision::Us,
        with_time_zone,
    }
}

#[test]
fn exe_2_1_the_parameter_at_position_n_is_bound_to_pn() {
    let parameters: Vec<Parameter> = (1..=3)
        .rev()
        .map(|position| Parameter {
            position,
            logical_type: integer(IntegerBits::ThirtyTwo, true),
            value: json!(position * 10),
        })
        .collect();
    let texts = parameter_texts(&parameters).unwrap();
    assert_eq!(
        texts,
        [
            ("p1".to_owned(), "10".to_owned()),
            ("p2".to_owned(), "20".to_owned()),
            ("p3".to_owned(), "30".to_owned()),
        ]
    );
}

#[test]
fn exe_2_2_null_is_bound_as_backslash_n() {
    for logical_type in [
        LogicalType::Boolean,
        LogicalType::Text,
        LogicalType::Uuid,
        timestamp(true),
    ] {
        assert_eq!(text(logical_type, Json::Null).unwrap(), "\\N");
    }
}

#[test]
fn exe_2_3_boolean_is_true_or_false() {
    assert_eq!(text(LogicalType::Boolean, json!(true)).unwrap(), "true");
    assert_eq!(text(LogicalType::Boolean, json!(false)).unwrap(), "false");
}

#[test]
fn exe_2_3_integer_is_its_decimal_digits() {
    assert_eq!(
        text(integer(IntegerBits::Eight, true), json!(-128)).unwrap(),
        "-128"
    );
    assert_eq!(
        text(
            integer(IntegerBits::OneHundredTwentyEight, true),
            json!("-170141183460469231731687303715884105728")
        )
        .unwrap(),
        "-170141183460469231731687303715884105728"
    );
    assert_eq!(
        text(
            integer(IntegerBits::OneHundredTwentyEight, false),
            json!("340282366920938463463374607431768211455")
        )
        .unwrap(),
        "340282366920938463463374607431768211455"
    );
    assert!(text(integer(IntegerBits::SixtyFour, true), json!("+1")).is_err());
}

#[test]
fn exe_2_3_decimal_is_the_con_4_string_unchanged() {
    let decimal = LogicalType::Decimal {
        precision: 38,
        scale: 10,
    };
    assert_eq!(
        text(decimal, json!("-1234567890123456789012345678.0123456789")).unwrap(),
        "-1234567890123456789012345678.0123456789"
    );
}

#[test]
fn exe_2_3_float_reads_back_as_exactly_its_value() {
    let single = text(
        LogicalType::Float {
            bits: FloatBits::ThirtyTwo,
        },
        json!(0.1),
    )
    .unwrap();
    assert_eq!(single, "0.1");
    assert_eq!(single.parse::<f32>().unwrap().to_bits(), 0.1_f32.to_bits());
    let double = text(
        LogicalType::Float {
            bits: FloatBits::SixtyFour,
        },
        json!(0.1),
    )
    .unwrap();
    assert_eq!(double.parse::<f64>().unwrap().to_bits(), 0.1_f64.to_bits());
    let large = text(
        LogicalType::Float {
            bits: FloatBits::SixtyFour,
        },
        json!(1.797_693_134_862_315_7e308),
    )
    .unwrap();
    assert_eq!(large.parse::<f64>().unwrap().to_bits(), f64::MAX.to_bits());
}

#[test]
fn exe_2_3_text_is_written_in_the_escaped_form() {
    assert_eq!(
        text(LogicalType::Text, json!("a\\b\tc\nd\re'f")).unwrap(),
        "a\\\\b\\tc\\nd\\re'f"
    );
}

#[test]
fn exe_2_3_date_is_the_con_4_text() {
    assert_eq!(
        text(LogicalType::Date, json!("1900-01-01")).unwrap(),
        "1900-01-01"
    );
}

#[test]
fn exe_2_3_time_and_timestamp_without_time_zone_have_no_clickhouse_value() {
    assert!(text(LogicalType::Time, json!("12:00:00")).is_err());
    assert!(text(timestamp(false), json!("2026-01-01T00:00:00")).is_err());
}

#[test]
fn exe_2_3_timestamp_with_time_zone_drops_the_t_and_the_z() {
    assert_eq!(
        text(timestamp(true), json!("2026-01-31T15:00:00.123456Z")).unwrap(),
        "2026-01-31 15:00:00.123456"
    );
}

#[test]
fn exe_2_3_uuid_is_the_con_4_text() {
    let uuid = "0189f7a2-6c1e-7cc3-8a6f-1d2e3f405162";
    assert_eq!(text(LogicalType::Uuid, json!(uuid)).unwrap(), uuid);
}

#[test]
fn exe_2_3_list_writes_null_numbers_and_quoted_texts() {
    let numbers = LogicalType::List {
        element: Box::new(integer(IntegerBits::SixtyFour, true)),
    };
    assert_eq!(text(numbers, json!([1, null, -2])).unwrap(), "[1,NULL,-2]");
    let booleans = LogicalType::List {
        element: Box::new(LogicalType::Boolean),
    };
    assert_eq!(
        text(booleans, json!([true, false])).unwrap(),
        "[true,false]"
    );
    let texts = LogicalType::List {
        element: Box::new(LogicalType::Text),
    };
    assert_eq!(
        text(texts, json!(["it's", "a\tb"])).unwrap(),
        "['it\\'s','a\\tb']"
    );
    let dates = LogicalType::List {
        element: Box::new(LogicalType::Date),
    };
    assert_eq!(
        text(dates, json!(["2026-01-01"])).unwrap(),
        "['2026-01-01']"
    );
}

#[test]
fn exe_2_3_binary_and_interval_are_never_bound() {
    assert!(text(LogicalType::Binary, json!("x")).is_err());
    assert!(text(LogicalType::Interval, json!("x")).is_err());
}

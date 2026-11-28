//! The parameter bindings of the reference executor on `DuckDB` (EXE-2): each row of the
//! EXE-2.3 table is bound and read back through the cast the plan writes (E6-4.2c), in the
//! non-default session of CS-4.2, and must denote exactly its CON-4 value.

use duckdb::types::{Decimal, TimeUnit, Value};
use menura_contracts::types::common::{
    FloatBits, IntegerBits, LogicalType, Parameter, TimestampPrecision,
};
use serde_json::{Value as Json, json};

use super::{bound_value, parameter_values};
use crate::database::prepare_database;
use crate::fixture::ModelFixture;

/// Whether the boolean statement `sql` holds with `value` bound to `$1`, on a fresh database
/// with the session settings of CS-4.2 (a non-UTC `TimeZone`).
fn holds(sql: &str, value: &Value) -> bool {
    let fixture = ModelFixture {
        sources: Vec::new(),
        data_script: String::new(),
    };
    let connection = prepare_database(&fixture).unwrap();
    connection
        .query_row(sql, [value], |row| row.get::<_, Option<bool>>(0))
        .unwrap()
        .unwrap_or(false)
}

/// The bound value of the CON-4 value `value` of `logical_type`, which must be bindable.
fn bound(logical_type: &LogicalType, value: &Json) -> Value {
    bound_value(logical_type, value).unwrap()
}

fn integer(bits: IntegerBits, signed: bool) -> LogicalType {
    LogicalType::Integer { bits, signed }
}

fn timestamp(precision: TimestampPrecision, with_time_zone: bool) -> LogicalType {
    LogicalType::Timestamp {
        precision,
        with_time_zone,
    }
}

#[test]
fn exe_2_1_the_parameter_at_position_n_is_bound_to_dollar_n() {
    let parameters = [
        Parameter {
            position: 2,
            logical_type: LogicalType::Text,
            value: json!("b"),
        },
        Parameter {
            position: 1,
            logical_type: LogicalType::Text,
            value: json!("a"),
        },
    ];
    let values = parameter_values(&parameters).unwrap();
    let fixture = ModelFixture {
        sources: Vec::new(),
        data_script: String::new(),
    };
    let connection = prepare_database(&fixture).unwrap();
    let joined: String = connection
        .query_row(
            "SELECT CAST($1 AS VARCHAR) || CAST($2 AS VARCHAR)",
            duckdb::params_from_iter(values.iter()),
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(joined, "ab");
}

#[test]
fn exe_2_2_null_is_bound_as_sql_null_whatever_the_type() {
    for logical_type in [
        LogicalType::Boolean,
        integer(IntegerBits::SixtyFour, true),
        LogicalType::Text,
        LogicalType::Uuid,
        LogicalType::Time,
        timestamp(TimestampPrecision::Us, true),
    ] {
        let value = bound(&logical_type, &Json::Null);
        assert_eq!(value, Value::Null, "{logical_type:?}");
        assert!(holds("SELECT CAST($1 AS VARCHAR) IS NULL", &value));
    }
}

#[test]
fn exe_2_3_boolean() {
    let value = bound(&LogicalType::Boolean, &json!(true));
    assert_eq!(value, Value::Boolean(true));
    assert!(holds("SELECT CAST($1 AS BOOLEAN) = true", &value));
}

#[test]
fn exe_2_3_integer_of_every_width_from_the_exact_integer() {
    let rows = [
        (
            integer(IntegerBits::Eight, true),
            json!(-128),
            "TINYINT",
            "-128",
        ),
        (
            integer(IntegerBits::Sixteen, false),
            json!(65_535),
            "USMALLINT",
            "65535",
        ),
        (
            integer(IntegerBits::SixtyFour, true),
            json!("-9223372036854775808"),
            "BIGINT",
            "-9223372036854775808",
        ),
        (
            integer(IntegerBits::SixtyFour, false),
            json!("18446744073709551615"),
            "UBIGINT",
            "18446744073709551615",
        ),
        (
            integer(IntegerBits::OneHundredTwentyEight, true),
            json!("-170141183460469231731687303715884105728"),
            "HUGEINT",
            "-170141183460469231731687303715884105728",
        ),
        (
            integer(IntegerBits::OneHundredTwentyEight, false),
            json!("340282366920938463463374607431768211455"),
            "UHUGEINT",
            "340282366920938463463374607431768211455",
        ),
    ];
    for (logical_type, value, duckdb_type, text) in rows {
        let value = bound(&logical_type, &value);
        let sql = format!("SELECT CAST($1 AS {duckdb_type}) = CAST('{text}' AS {duckdb_type})");
        assert!(holds(&sql, &value), "{sql}");
    }
    assert!(bound_value(&integer(IntegerBits::Eight, false), &json!(-1)).is_err());
    assert!(bound_value(&integer(IntegerBits::SixtyFour, true), &json!("+1")).is_err());
}

#[test]
fn exe_2_3_decimal_from_the_unscaled_integer() {
    let decimal = LogicalType::Decimal {
        precision: 38,
        scale: 10,
    };
    let text = "-1234567890123456789012345678.0123456789";
    let value = bound(&decimal, &json!(text));
    assert!(matches!(value, Value::Decimal(_)));
    let sql = format!("SELECT CAST($1 AS DECIMAL(38,10)) = CAST('{text}' AS DECIMAL(38,10))");
    assert!(holds(&sql, &value), "{sql}");
}

#[test]
fn exe_2_3_float_of_32_and_64_bits() {
    let single = bound(
        &LogicalType::Float {
            bits: FloatBits::ThirtyTwo,
        },
        &json!(0.1),
    );
    assert_eq!(single, Value::Float(0.1_f32));
    assert!(holds(
        "SELECT CAST($1 AS FLOAT) = CAST('0.1' AS FLOAT)",
        &single
    ));
    let double = bound(
        &LogicalType::Float {
            bits: FloatBits::SixtyFour,
        },
        &json!(0.1),
    );
    assert_eq!(double, Value::Double(0.1));
    assert!(holds(
        "SELECT CAST($1 AS DOUBLE) = CAST('0.1' AS DOUBLE)",
        &double
    ));
}

#[test]
fn exe_2_3_text() {
    let value = bound(&LogicalType::Text, &json!("it's\ta\\b\n"));
    assert!(holds(
        "SELECT CAST($1 AS VARCHAR) = 'it''s' || chr(9) || 'a\\b' || chr(10)",
        &value
    ));
}

#[test]
fn exe_2_3_date_of_the_proleptic_gregorian_calendar() {
    for day in ["0001-01-01", "1582-10-10", "2024-02-29", "9999-12-31"] {
        let value = bound(&LogicalType::Date, &json!(day));
        let sql = format!("SELECT CAST($1 AS DATE) = DATE '{day}'");
        assert!(holds(&sql, &value), "{sql}");
    }
}

#[test]
fn exe_2_3_time_of_the_microseconds_since_midnight() {
    let value = bound(&LogicalType::Time, &json!("23:59:59.999999"));
    assert_eq!(value, Value::Time64(TimeUnit::Microsecond, 86_399_999_999));
    assert!(holds(
        "SELECT CAST($1 AS TIME) = TIME '23:59:59.999999'",
        &value
    ));
}

#[test]
fn exe_2_3_timestamp_without_time_zone_is_the_wall_clock_value() {
    let value = bound(
        &timestamp(TimestampPrecision::Us, false),
        &json!("2026-03-08T02:30:00.5"),
    );
    assert!(matches!(value, Value::Timestamp(TimeUnit::Microsecond, _)));
    assert!(holds(
        "SELECT CAST($1 AS TIMESTAMP) = TIMESTAMP '2026-03-08 02:30:00.5'",
        &value
    ));
}

#[test]
fn exe_2_3_timestamp_with_time_zone_is_the_instant_whatever_the_session_time_zone() {
    let value = bound(
        &timestamp(TimestampPrecision::Us, true),
        &json!("2026-01-31T15:00:00.123456Z"),
    );
    assert_eq!(
        value,
        Value::Text("2026-01-31T15:00:00.123456+00:00".to_owned())
    );
    assert!(holds(
        "SELECT CAST($1 AS TIMESTAMP WITH TIME ZONE) = TIMESTAMPTZ '2026-01-31 15:00:00.123456+00'",
        &value
    ));
}

#[test]
fn exe_2_3_uuid_is_bound_as_its_text() {
    let text = "0189f7a2-6c1e-7cc3-8a6f-1d2e3f405162";
    let value = bound(&LogicalType::Uuid, &json!(text));
    assert_eq!(value, Value::Text(text.to_owned()));
    let sql = format!("SELECT CAST($1 AS UUID) = UUID '{text}'");
    assert!(holds(&sql, &value), "{sql}");
    for invalid in [
        json!("0189F7A2-6C1E-7CC3-8A6F-1D2E3F405162"),
        json!("0189f7a26c1e7cc38a6f1d2e3f405162"),
        json!("{0189f7a2-6c1e-7cc3-8a6f-1d2e3f405162}"),
        json!(7),
    ] {
        assert!(
            bound_value(&LogicalType::Uuid, &invalid).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn exe_2_3_list_elements_are_bound_as_their_type_and_null_as_null() {
    let list = LogicalType::List {
        element: Box::new(integer(IntegerBits::ThirtyTwo, true)),
    };
    let value = bound(&list, &json!([1, null, -3]));
    assert_eq!(
        value,
        Value::List(vec![Value::Int(1), Value::Null, Value::Int(-3)])
    );
    assert!(holds(
        "SELECT CAST($1 AS INTEGER[]) IS NOT DISTINCT FROM [1, NULL, -3]",
        &value
    ));
}

#[test]
fn exe_2_3_binary_and_interval_are_never_bound() {
    for logical_type in [LogicalType::Binary, LogicalType::Interval] {
        assert!(
            bound_value(&logical_type, &json!("x")).is_err(),
            "{logical_type:?}"
        );
    }
}

#[test]
fn exe_2_4_no_parameter_is_nan_or_infinite() {
    for number in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(serde_json::Number::from_f64(number).is_none());
    }
    let float = LogicalType::Float {
        bits: FloatBits::SixtyFour,
    };
    assert!(bound_value(&float, &json!("NaN")).is_err());
}

#[test]
fn exe_2_5_six_fraction_digits_bind_exactly_at_microsecond_resolution() {
    let time = bound(&LogicalType::Time, &json!("00:00:00.000001"));
    assert_eq!(time, Value::Time64(TimeUnit::Microsecond, 1));
    for precision in [TimestampPrecision::Us, TimestampPrecision::Ns] {
        let value = bound(
            &timestamp(precision, false),
            &json!("1970-01-01T00:00:00.123456"),
        );
        assert_eq!(value, Value::Timestamp(TimeUnit::Microsecond, 123_456));
    }
    let nanoseconds = bound(
        &timestamp(TimestampPrecision::Ns, false),
        &json!("2026-01-31T15:00:00.123456"),
    );
    assert!(holds(
        "SELECT CAST($1 AS TIMESTAMP_NS) = TIMESTAMP_NS '2026-01-31 15:00:00.123456'",
        &nanoseconds
    ));
}

#[test]
fn exe_2_5_more_than_six_fraction_digits_are_refused_never_truncated() {
    assert!(bound_value(&LogicalType::Time, &json!("00:00:00.0000001")).is_err());
    assert!(
        bound_value(
            &timestamp(TimestampPrecision::Ns, false),
            &json!("1970-01-01T00:00:00.123456789")
        )
        .is_err()
    );
    assert!(
        bound_value(
            &timestamp(TimestampPrecision::Ns, true),
            &json!("1970-01-01T00:00:00.123456789Z")
        )
        .is_err()
    );
}

#[test]
fn exe_2_3_decimal_value_is_never_a_double() {
    let decimal = LogicalType::Decimal {
        precision: 18,
        scale: 2,
    };
    assert_eq!(
        bound(&decimal, &json!("0.10")),
        Value::Decimal(Decimal::new(18, 2, 10).unwrap())
    );
}

//! Binds the parameters of a compiled plan (PLN-1): each CON-4 value becomes the `DuckDB` value
//! of its `logical_type`, so the engine never guesses a parameter's type from its text.

use duckdb::types::{Decimal, TimeUnit, Value};
use menura_contracts::types::common::{FloatBits, IntegerBits, LogicalType, Parameter};
use serde_json::Value as Json;

use crate::calendar::{parse_date, parse_time, parse_timestamp};

/// The values of `parameters`, in position order (`$1`, `$2`, …).
///
/// # Errors
///
/// Returns why a parameter cannot be bound: its positions are not 1, 2, …, its value does not
/// have the CON-4 encoding of its type, or the runner cannot bind its type.
pub(crate) fn parameter_values(parameters: &[Parameter]) -> Result<Vec<Value>, String> {
    let mut ordered: Vec<&Parameter> = parameters.iter().collect();
    ordered.sort_by_key(|parameter| parameter.position);
    ordered
        .iter()
        .zip(1_u64..)
        .map(|(parameter, expected)| {
            if parameter.position != expected {
                return Err(format!("no parameter has the position {expected}"));
            }
            bound_value(&parameter.logical_type, &parameter.value)
                .map_err(|reason| format!("parameter ${expected}: {reason}"))
        })
        .collect()
}

/// The `DuckDB` value of the CON-4 value `value` of type `logical_type`.
fn bound_value(logical_type: &LogicalType, value: &Json) -> Result<Value, String> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let invalid = || format!("`{value}` is not a CON-4 value of type {logical_type:?}");
    match logical_type {
        LogicalType::Boolean => value.as_bool().map(Value::Boolean).ok_or_else(invalid),
        LogicalType::Integer { bits, signed } => {
            bound_integer(value, *bits, *signed).ok_or_else(invalid)
        }
        LogicalType::Decimal { precision, scale } => {
            decimal_value(value.as_str().ok_or_else(invalid)?, *precision, *scale)
                .ok_or_else(invalid)
        }
        LogicalType::Float { bits } => Ok(float_value(value.as_f64().ok_or_else(invalid)?, *bits)),
        LogicalType::Text => Ok(Value::Text(value.as_str().ok_or_else(invalid)?.to_owned())),
        LogicalType::Date | LogicalType::Time | LogicalType::Timestamp { .. } => {
            temporal_value(logical_type, value.as_str().ok_or_else(invalid)?).ok_or_else(invalid)
        }
        LogicalType::List { element } => value
            .as_array()
            .ok_or_else(invalid)?
            .iter()
            .map(|item| bound_value(element, item))
            .collect::<Result<_, _>>()
            .map(Value::List),
        LogicalType::Uuid => uuid_value(value.as_str().ok_or_else(invalid)?).ok_or_else(invalid),
        LogicalType::Interval | LogicalType::Binary | LogicalType::Unsupported => Err(format!(
            "the runner cannot bind a parameter of type {logical_type:?}"
        )),
    }
}

/// A `uuid` (CON-4: lowercase and hyphenated), bound as a `VARCHAR` holding its text, which
/// the plan's `CAST($n AS UUID)` reads exactly (EXE-2.3).
fn uuid_value(text: &str) -> Option<Value> {
    let groups: Vec<&str> = text.split('-').collect();
    let lengths: Vec<usize> = groups.iter().map(|group| group.len()).collect();
    let lowercase_hex = text
        .bytes()
        .all(|byte| byte == b'-' || byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    (lengths == [8, 4, 4, 4, 12] && lowercase_hex).then(|| Value::Text(text.to_owned()))
}

/// The number of fraction digits a binding at microsecond resolution keeps exactly (EXE-2.5).
const MAXIMUM_FRACTION_DIGITS: usize = 6;

/// Whether the seconds of the time or timestamp `text` have at most six fraction digits, so
/// that a binding at microsecond resolution neither rounds nor truncates it (EXE-2.1, EXE-2.5).
fn has_exact_fraction(text: &str) -> bool {
    let without_zone = text.strip_suffix('Z').unwrap_or(text);
    without_zone
        .split_once('.')
        .is_none_or(|(_, fraction)| fraction.len() <= MAXIMUM_FRACTION_DIGITS)
}

/// The `DuckDB` value of a date, a time or a timestamp in its CON-4 text form.
fn temporal_value(logical_type: &LogicalType, text: &str) -> Option<Value> {
    if !has_exact_fraction(text) {
        return None;
    }
    match logical_type {
        LogicalType::Date => i32::try_from(parse_date(text)?).ok().map(Value::Date32),
        LogicalType::Time => {
            parse_time(text).map(|micros| Value::Time64(TimeUnit::Microsecond, micros))
        }
        LogicalType::Timestamp {
            with_time_zone: false,
            ..
        } => parse_timestamp(text, false)
            .map(|micros| Value::Timestamp(TimeUnit::Microsecond, micros)),
        LogicalType::Timestamp {
            with_time_zone: true,
            ..
        } => instant_value(text),
        _ => None,
    }
}

/// A timestamp with time zone (CON-4: in UTC, with a final `Z`). The bindings have no value
/// of that type, and a `TIMESTAMP` value would be converted in the session `TimeZone`, so it is
/// bound as its text with the explicit offset `+00:00`, which `DuckDB` converts to the same
/// instant whatever the session settings (CS-4.2).
fn instant_value(text: &str) -> Option<Value> {
    parse_timestamp(text, true)?;
    let utc = text.strip_suffix('Z')?;
    Some(Value::Text(format!("{utc}+00:00")))
}

/// The `DuckDB` integer of a CON-4 integer of the parameter's type, made from the exact
/// integer (EXE-2.3). An unsigned 128-bit integer may exceed the signed range, so it is read
/// on its own.
fn bound_integer(value: &Json, bits: IntegerBits, signed: bool) -> Option<Value> {
    if bits == IntegerBits::OneHundredTwentyEight && !signed {
        return unsigned_integer_of(value).map(Value::UHugeInt);
    }
    integer_value(integer_of(value)?, bits, signed)
}

/// An integer written as a JSON number or, outside ±(2^53 − 1), as a string (CON-4).
fn integer_of(value: &Json) -> Option<i128> {
    match value {
        Json::Number(number) => number.as_i64().map(i128::from),
        Json::String(text) if !text.starts_with('+') => text.parse().ok(),
        _ => None,
    }
}

/// A non-negative integer written as a JSON number or as a string (CON-4), up to 2^128 − 1.
fn unsigned_integer_of(value: &Json) -> Option<u128> {
    match value {
        Json::Number(number) => number.as_u64().map(u128::from),
        Json::String(text) if !text.starts_with('+') => text.parse().ok(),
        _ => None,
    }
}

/// The `DuckDB` integer of the width and signedness of the parameter's type.
fn integer_value(integer: i128, bits: IntegerBits, signed: bool) -> Option<Value> {
    Some(match (bits, signed) {
        (IntegerBits::Eight, true) => Value::TinyInt(integer.try_into().ok()?),
        (IntegerBits::Sixteen, true) => Value::SmallInt(integer.try_into().ok()?),
        (IntegerBits::ThirtyTwo, true) => Value::Int(integer.try_into().ok()?),
        (IntegerBits::SixtyFour, true) => Value::BigInt(integer.try_into().ok()?),
        (IntegerBits::OneHundredTwentyEight, true) => Value::HugeInt(integer),
        (IntegerBits::Eight, false) => Value::UTinyInt(integer.try_into().ok()?),
        (IntegerBits::Sixteen, false) => Value::USmallInt(integer.try_into().ok()?),
        (IntegerBits::ThirtyTwo, false) => Value::UInt(integer.try_into().ok()?),
        (IntegerBits::SixtyFour, false) => Value::UBigInt(integer.try_into().ok()?),
        (IntegerBits::OneHundredTwentyEight, false) => Value::UHugeInt(integer.try_into().ok()?),
    })
}

/// The `DuckDB` decimal of a CON-4 decimal string with exactly `scale` fraction digits.
fn decimal_value(text: &str, precision: u8, scale: u8) -> Option<Value> {
    let (negative, unsigned) = text
        .strip_prefix('-')
        .map_or((false, text), |rest| (true, rest));
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let all_digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    if whole.is_empty() || !all_digits(whole) || !all_digits(fraction) {
        return None;
    }
    if fraction.len() != usize::from(scale) {
        return None;
    }
    let magnitude: i128 = format!("{whole}{fraction}").parse().ok()?;
    let scaled = if negative { -magnitude } else { magnitude };
    Decimal::new(precision, scale, scaled)
        .ok()
        .map(Value::Decimal)
}

/// The `DuckDB` float of the width of the parameter's type.
fn float_value(number: f64, bits: FloatBits) -> Value {
    match bits {
        #[allow(
            clippy::cast_possible_truncation,
            reason = "a 32-bit float parameter holds a value of 32-bit precision"
        )]
        FloatBits::ThirtyTwo => Value::Float(number as f32),
        FloatBits::SixtyFour => Value::Double(number),
    }
}

#[cfg(test)]
mod executor_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn parameter(position: u64, logical_type: LogicalType, value: Json) -> Parameter {
        Parameter {
            position,
            logical_type,
            value,
        }
    }

    fn integer(bits: IntegerBits) -> LogicalType {
        LogicalType::Integer { bits, signed: true }
    }

    #[test]
    fn pln_1_parameters_are_bound_in_position_order() {
        let parameters = [
            parameter(2, LogicalType::Text, Json::from("NORTH")),
            parameter(1, integer(IntegerBits::SixtyFour), Json::from(1)),
        ];
        assert_eq!(
            parameter_values(&parameters).unwrap(),
            [Value::BigInt(1), Value::Text("NORTH".to_owned())]
        );
    }

    #[test]
    fn pln_1_a_missing_position_is_reported() {
        let parameters = [parameter(2, LogicalType::Text, Json::from("x"))];
        assert!(parameter_values(&parameters).is_err());
    }

    #[test]
    fn con_4_integers_take_the_width_of_their_type() {
        let huge = LogicalType::Integer {
            bits: IntegerBits::OneHundredTwentyEight,
            signed: true,
        };
        assert_eq!(
            bound_value(&huge, &Json::from("-9007199254740993")).unwrap(),
            Value::HugeInt(-9_007_199_254_740_993)
        );
        assert_eq!(
            bound_value(&integer(IntegerBits::ThirtyTwo), &Json::from(7)).unwrap(),
            Value::Int(7)
        );
        assert!(bound_value(&integer(IntegerBits::Eight), &Json::from(300)).is_err());
    }

    #[test]
    fn con_4_decimals_need_exactly_their_scale() {
        let decimal = LogicalType::Decimal {
            precision: 3,
            scale: 2,
        };
        assert_eq!(
            bound_value(&decimal, &Json::from("-1.50")).unwrap(),
            Value::Decimal(Decimal::new(3, 2, -150).unwrap())
        );
        assert!(bound_value(&decimal, &Json::from("1.5")).is_err());
        assert!(bound_value(&decimal, &Json::from(1.5)).is_err());
    }

    #[test]
    fn con_4_dates_times_and_timestamps_are_bound_as_such() {
        assert_eq!(
            bound_value(&LogicalType::Date, &Json::from("1970-01-02")).unwrap(),
            Value::Date32(1)
        );
        assert_eq!(
            bound_value(&LogicalType::Time, &Json::from("00:00:01")).unwrap(),
            Value::Time64(TimeUnit::Microsecond, 1_000_000)
        );
        let timestamp = LogicalType::Timestamp {
            precision: menura_contracts::types::common::TimestampPrecision::Us,
            with_time_zone: false,
        };
        assert_eq!(
            bound_value(&timestamp, &Json::from("1970-01-01T00:00:01")).unwrap(),
            Value::Timestamp(TimeUnit::Microsecond, 1_000_000)
        );
    }

    #[test]
    fn con_4_an_instant_is_bound_as_utc_text_with_an_explicit_offset() {
        let instant = LogicalType::Timestamp {
            precision: menura_contracts::types::common::TimestampPrecision::Us,
            with_time_zone: true,
        };
        assert_eq!(
            bound_value(&instant, &Json::from("2026-01-31T15:00:00.5Z")).unwrap(),
            Value::Text("2026-01-31T15:00:00.5+00:00".to_owned())
        );
        assert!(bound_value(&instant, &Json::from("2026-01-31T15:00:00")).is_err());
    }

    #[test]
    fn e6_4_10_an_instant_parameter_is_the_same_instant_in_any_session_time_zone() {
        let fixture = crate::fixture::ModelFixture {
            sources: Vec::new(),
            data_script: String::new(),
        };
        let connection = crate::database::prepare_database(&fixture).unwrap();
        let instant = LogicalType::Timestamp {
            precision: menura_contracts::types::common::TimestampPrecision::Us,
            with_time_zone: true,
        };
        let value = bound_value(&instant, &Json::from("2026-01-31T15:00:00Z")).unwrap();
        for sql in [
            "SELECT CAST($1 AS TIMESTAMP WITH TIME ZONE) = TIMESTAMPTZ '2026-01-31 15:00:00+00'",
            "SELECT TIMESTAMPTZ '2026-01-31 15:00:00+00' = $1",
        ] {
            let equal: bool = connection
                .query_row(sql, [&value], |row| row.get(0))
                .unwrap();
            assert!(equal, "{sql}");
        }
    }

    #[test]
    fn e6_5_3_list_parameters_are_bound_as_lists_of_their_element_type() {
        let list = LogicalType::List {
            element: Box::new(LogicalType::Text),
        };
        assert_eq!(
            bound_value(&list, &serde_json::json!(["a", null])).unwrap(),
            Value::List(vec![Value::Text("a".to_owned()), Value::Null])
        );
        assert_eq!(
            bound_value(&list, &serde_json::json!([])).unwrap(),
            Value::List(Vec::new())
        );
    }

    #[test]
    fn con_4_null_is_bound_as_null_whatever_the_type() {
        assert_eq!(
            bound_value(&LogicalType::Date, &Json::Null).unwrap(),
            Value::Null
        );
    }
}

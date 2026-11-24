//! Values in the SQL text (E6-4.11) and in the parameters, encoded as CON-4 requires.

use menura_contracts::encode_integer;
use menura_model::expression::{
    DateValue, DecimalLiteral, FloatLiteral, IntegerLiteral, Literal, TimeOfDay, TimestampValue,
};
use menura_security::{AdmittedScalar, AdmittedValue};
use serde_json::Value;

/// The SQL text of a model constant (E6-4.11), or `None` for a `time` or a `timestamp`
/// without time zone, which `ClickHouse` cannot express (E6-7.12). Each constant is written
/// with its E0.f type: an integer as an `Int64`, where `ClickHouse` would give a literal the
/// smallest type that holds it, and a decimal cast from its exact text, so that `ClickHouse`
/// never reads it as a float.
pub(crate) fn literal_sql(literal: &Literal) -> Option<String> {
    let text = match literal {
        Literal::Null => "NULL".to_owned(),
        Literal::Boolean(true) => "TRUE".to_owned(),
        Literal::Boolean(false) => "FALSE".to_owned(),
        Literal::Text(text) => text_sql(text),
        Literal::Integer(number) if is_big_integer(number) => {
            format!("toInt64({})", integer_text(number.digits()))
        }
        Literal::Integer(number) => format!(
            "CAST('{}' AS Decimal({}, 0))",
            integer_text(number.digits()),
            number.digits().len()
        ),
        Literal::Decimal(number) => format!(
            "CAST('{}' AS Decimal({}, {}))",
            decimal_text(number),
            number.precision(),
            number.scale()
        ),
        Literal::Float(number) => float_text(number),
        Literal::Date(date) => format!("toDate32('{}')", date_text(*date)),
        Literal::Time(_) | Literal::Timestamp(_) => return None,
    };
    Some(text)
}

/// A text constant between single quotes, with every backslash and every single quote
/// doubled: `ClickHouse` reads a backslash in a string literal as an escape character.
pub(crate) fn text_sql(text: &str) -> String {
    format!("'{}'", text.replace('\\', "\\\\").replace('\'', "''"))
}

/// The CON-4 value of a query value, or `None` for a float without a finite value. `NULL` is
/// JSON `null`.
pub(crate) fn literal_value(literal: &Literal) -> Option<Value> {
    let value = match literal {
        Literal::Null => Value::Null,
        Literal::Boolean(value) => Value::Bool(*value),
        Literal::Text(text) => Value::String(text.clone()),
        Literal::Integer(number) if is_big_integer(number) => {
            encode_integer(number.digits().parse().ok()?)
        }
        Literal::Integer(number) => Value::String(integer_text(number.digits())),
        Literal::Decimal(number) => Value::String(decimal_text(number)),
        Literal::Float(number) => Value::from(serde_json::Number::from_f64(
            float_text(number).parse().ok()?,
        )?),
        Literal::Date(date) => Value::String(date_text(*date)),
        Literal::Time(time) => Value::String(time_text(time)),
        Literal::Timestamp(timestamp) => Value::String(timestamp_text(timestamp)),
    };
    Some(value)
}

/// The CON-4 value of an admitted attribute: a list is a JSON array of its items.
pub(crate) fn attribute_value(value: &AdmittedValue) -> Value {
    match value {
        AdmittedValue::Scalar(scalar) => scalar_value(scalar),
        AdmittedValue::List(items) => Value::Array(items.iter().map(scalar_value).collect()),
    }
}

fn scalar_value(scalar: &AdmittedScalar) -> Value {
    match scalar {
        AdmittedScalar::Text(text) => Value::String(text.clone()),
        AdmittedScalar::Integer(number) => encode_integer(i128::from(*number)),
        AdmittedScalar::Decimal(number) => Value::String(number.to_string()),
        AdmittedScalar::Date(date) => Value::String(date.to_string()),
        AdmittedScalar::Boolean(value) => Value::Bool(*value),
    }
}

/// Whether an integer literal is a 64-bit integer rather than a `decimal(n,0)` (E0F-9.3).
fn is_big_integer(number: &IntegerLiteral) -> bool {
    number.digits().len() <= IntegerLiteral::MAXIMUM_BIG_INTEGER_DIGITS
}

/// Digits without leading zeros; `0` for zero.
fn integer_text(digits: &str) -> String {
    let trimmed = digits.trim_start_matches('0');
    if trimmed.is_empty() {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// The CON-4 canonical decimal: integer digits without leading zeros, then `.` and exactly
/// `scale` digits when the scale is not zero.
fn decimal_text(number: &DecimalLiteral) -> String {
    let integer = integer_text(number.integer_digits());
    if number.fraction_digits().is_empty() {
        integer
    } else {
        format!("{integer}.{}", number.fraction_digits())
    }
}

/// The float in scientific notation, which `ClickHouse` reads as a `Float64`.
fn float_text(number: &FloatLiteral) -> String {
    let integer = integer_text(number.integer_digits());
    let fraction = number.fraction_digits();
    if fraction.is_empty() {
        format!("{integer}e{}", number.exponent())
    } else {
        format!("{integer}.{fraction}e{}", number.exponent())
    }
}

pub(crate) fn date_text(date: DateValue) -> String {
    format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day())
}

/// `HH:MM:SS`, then `.` and the fraction without trailing zeros when it is not zero (CON-4).
fn time_text(time: &TimeOfDay) -> String {
    let base = format!(
        "{:02}:{:02}:{:02}",
        time.hour(),
        time.minute(),
        time.second()
    );
    let fraction = time.fraction_digits().trim_end_matches('0');
    if fraction.is_empty() {
        base
    } else {
        format!("{base}.{fraction}")
    }
}

/// A timestamp without time zone: `YYYY-MM-DDTHH:MM:SS` and the fraction rule (CON-4).
pub(crate) fn timestamp_text(timestamp: &TimestampValue) -> String {
    format!(
        "{}T{}",
        date_text(timestamp.date),
        time_text(&timestamp.time)
    )
}

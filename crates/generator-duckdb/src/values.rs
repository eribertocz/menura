//! Values in the SQL text (E6-4.11) and in the parameters, encoded as CON-4 requires.

use menura_contracts::encode_integer;
use menura_model::expression::{
    DateValue, DecimalLiteral, FloatLiteral, IntegerLiteral, Literal, TimeOfDay, TimestampValue,
};
use menura_model::internal_model::OpenEnd;
use menura_security::{AdmittedScalar, AdmittedValue};
use serde_json::Value;

/// The SQL text of a model constant (E6-4.11). Decimals are cast from their exact text, so
/// `DuckDB` never reads them as floats.
pub(crate) fn literal_sql(literal: &Literal) -> String {
    match literal {
        Literal::Null => "NULL".to_owned(),
        Literal::Boolean(true) => "TRUE".to_owned(),
        Literal::Boolean(false) => "FALSE".to_owned(),
        Literal::Text(text) => text_sql(text),
        Literal::Integer(number) if is_big_integer(number) => integer_text(number.digits()),
        Literal::Integer(number) => format!(
            "CAST('{}' AS DECIMAL({},0))",
            integer_text(number.digits()),
            number.digits().len()
        ),
        Literal::Decimal(number) => format!(
            "CAST('{}' AS DECIMAL({},{}))",
            decimal_text(number),
            number.precision(),
            number.scale()
        ),
        Literal::Float(number) => float_text(number),
        Literal::Date(date) => format!("DATE '{}'", date_text(*date)),
        Literal::Time(time) => format!("TIME '{}'", time_text(time)),
        Literal::Timestamp(timestamp) => format!(
            "TIMESTAMP '{} {}'",
            date_text(timestamp.date),
            time_text(&timestamp.time)
        ),
    }
}

/// The text of a model `open_end` (VAL-1.2) that `DuckDB` casts to its `physical` type: the
/// date, or the date and time separated by a space, with the offset `+00` for an instant.
pub(crate) fn open_end_text(open_end: &OpenEnd) -> String {
    match open_end {
        OpenEnd::Date(date) => text_sql(&date_text(*date)),
        OpenEnd::Timestamp(timestamp) => {
            let zone = if timestamp.with_time_zone { "+00" } else { "" };
            text_sql(&format!(
                "{} {}{zone}",
                date_text(timestamp.date),
                timestamp.time
            ))
        }
    }
}

/// A text constant between single quotes, with every single quote doubled.
pub(crate) fn text_sql(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
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

/// The float in scientific notation, which `DuckDB` reads as a `DOUBLE`.
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

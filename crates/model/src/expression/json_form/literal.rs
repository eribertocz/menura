//! The JSON form of literals: their value in the canonical form of CON-4 and their type as
//! written (E0H-1.3).

use super::super::literal::DateValue;
use super::super::literal::{
    DecimalLiteral, FloatLiteral, IntegerLiteral, Literal, TimeOfDay, TimestampValue,
};
use crate::json::JsonForm;

/// The largest integer written as a JSON number (CON-4).
const MAXIMUM_EXACT_NUMBER: u64 = (1 << 53) - 1;

/// `{"node": "literal", "type": …, "value": …}`, with `precision` and `scale` for exact
/// numbers, whose type depends on the digits as written.
pub(super) fn literal(literal: &Literal) -> JsonForm {
    let (kind, value) = match literal {
        Literal::Null => ("null", JsonForm::Null),
        Literal::Boolean(value) => ("boolean", JsonForm::Boolean(*value)),
        Literal::Text(text) => ("text", JsonForm::text(text)),
        Literal::Integer(number) => return integer(number),
        Literal::Decimal(number) => return decimal(number),
        Literal::Float(number) => ("float", float(number)),
        Literal::Date(date) => ("date", JsonForm::Text(date_text(*date))),
        Literal::Time(time) => ("time", JsonForm::Text(time_text(time))),
        Literal::Timestamp(timestamp) => ("timestamp", JsonForm::Text(timestamp_text(timestamp))),
    };
    JsonForm::object([
        ("node", JsonForm::text("literal")),
        ("type", JsonForm::text(kind)),
        ("value", value),
    ])
}

fn integer(number: &IntegerLiteral) -> JsonForm {
    let digits = without_leading_zeros(number.digits());
    let value = match digits.parse::<u64>() {
        Ok(value) if value <= MAXIMUM_EXACT_NUMBER => {
            i64::try_from(value).map_or_else(|_| JsonForm::text(digits), JsonForm::Integer)
        }
        _ => JsonForm::text(digits),
    };
    JsonForm::object([
        ("node", JsonForm::text("literal")),
        ("type", JsonForm::text("integer")),
        ("precision", count(number.digits().len())),
        ("value", value),
    ])
}

fn decimal(number: &DecimalLiteral) -> JsonForm {
    let integer = without_leading_zeros(number.integer_digits());
    let value = if number.fraction_digits().is_empty() {
        integer.to_owned()
    } else {
        format!("{integer}.{}", number.fraction_digits())
    };
    JsonForm::object([
        ("node", JsonForm::text("literal")),
        ("type", JsonForm::text("decimal")),
        ("precision", count(number.precision())),
        ("scale", count(number.scale())),
        ("value", JsonForm::Text(value)),
    ])
}

/// The float as the text of a JSON number: no leading zeros, and the fraction only when
/// written.
fn float(number: &FloatLiteral) -> JsonForm {
    let integer = without_leading_zeros(number.integer_digits());
    let fraction = number.fraction_digits();
    let separator = if fraction.is_empty() { "" } else { "." };
    JsonForm::Number(format!(
        "{integer}{separator}{fraction}e{}",
        number.exponent()
    ))
}

/// `digits` without leading zeros, or `0` when every digit is zero or there is none.
fn without_leading_zeros(digits: &str) -> &str {
    let trimmed = digits.trim_start_matches('0');
    if trimmed.is_empty() { "0" } else { trimmed }
}

fn count(value: usize) -> JsonForm {
    JsonForm::Integer(i64::try_from(value).unwrap_or(i64::MAX))
}

/// `YYYY-MM-DD` (CON-4).
pub(super) fn date_text(date: DateValue) -> String {
    format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day())
}

/// `HH:MM:SS`, with `.` and the fraction digits only when they are not all zero, trailing zeros
/// removed (CON-4).
fn time_text(time: &TimeOfDay) -> String {
    let fraction = time.fraction_digits().trim_end_matches('0');
    let separator = if fraction.is_empty() { "" } else { "." };
    format!(
        "{:02}:{:02}:{:02}{separator}{fraction}",
        time.hour(),
        time.minute(),
        time.second()
    )
}

/// `YYYY-MM-DDTHH:MM:SS` with the fraction rule of [`time_text`] (CON-4).
fn timestamp_text(timestamp: &TimestampValue) -> String {
    format!(
        "{}T{}",
        date_text(timestamp.date),
        time_text(&timestamp.time)
    )
}

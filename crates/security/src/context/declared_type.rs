//! SEC-3.3: an attribute value checked against its declared type, with the one encoding each
//! type accepts (CON-4).

use menura_contracts::types::security_context::{AttributeScalar, AttributeValue};
use menura_contracts::value_encoding::{
    is_canonical_date, is_canonical_decimal, is_canonical_integer_string,
};
use menura_model::internal_model::{AttributeType, ScalarType};

use super::value::{AdmittedScalar, AdmittedValue, DateValue, DecimalValue};

/// Why a value is not accepted, and where: `None` for the attribute itself, or the index of
/// the offending list item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Mismatch {
    pub(crate) index: Option<usize>,
    pub(crate) message: String,
}

/// Checks `value` against `declared`. A scalar is never accepted for a list, nor an array for
/// a scalar, and the items of a list are checked in index order.
pub(crate) fn check_value(
    value: &AttributeValue,
    declared: AttributeType,
) -> Result<AdmittedValue, Mismatch> {
    match (value, declared) {
        (AttributeValue::Scalar(scalar), AttributeType::Scalar(item)) => check_scalar(scalar, item)
            .map(AdmittedValue::Scalar)
            .map_err(|message| Mismatch {
                index: None,
                message,
            }),
        (AttributeValue::List(items), AttributeType::List(item)) => items
            .iter()
            .enumerate()
            .map(|(index, scalar)| {
                check_scalar(scalar, item).map_err(|message| Mismatch {
                    index: Some(index),
                    message,
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .map(AdmittedValue::List),
        (AttributeValue::List(_), AttributeType::Scalar(item)) => Err(Mismatch {
            index: None,
            message: format!(
                "the value is an array, but the attribute is declared `{}`",
                name(item)
            ),
        }),
        (AttributeValue::Scalar(_), AttributeType::List(item)) => Err(Mismatch {
            index: None,
            message: format!(
                "the value must be an array, because the attribute is declared `list<{}>`",
                name(item)
            ),
        }),
    }
}

/// Checks one scalar against a scalar type; the error is the English reason.
fn check_scalar(value: &AttributeScalar, declared: ScalarType) -> Result<AdmittedScalar, String> {
    let admitted = match (value, declared) {
        (AttributeScalar::Text(text), ScalarType::Text) => Some(AdmittedScalar::Text(text.clone())),
        (AttributeScalar::Integer(integer), ScalarType::Integer) => {
            Some(AdmittedScalar::Integer(*integer))
        }
        (AttributeScalar::Text(text), ScalarType::Integer) => {
            large_integer(text).map(AdmittedScalar::Integer)
        }
        (AttributeScalar::Text(text), ScalarType::Decimal { precision, scale }) => {
            decimal(text, precision, scale).map(AdmittedScalar::Decimal)
        }
        (AttributeScalar::Text(text), ScalarType::Date) => date(text).map(AdmittedScalar::Date),
        (AttributeScalar::Boolean(boolean), ScalarType::Boolean) => {
            Some(AdmittedScalar::Boolean(*boolean))
        }
        _ => None,
    };
    admitted.ok_or_else(|| format!("the value is not {}", expected(declared)))
}

/// An integer outside ±(2^53 − 1) written as its canonical string, within 64 bits.
fn large_integer(text: &str) -> Option<i64> {
    is_canonical_integer_string(text)
        .then(|| text.parse().ok())
        .flatten()
}

/// A canonical decimal with exactly `scale` fraction digits and at most `precision − scale`
/// integer digits. The single `0` of a value below one is not an integer digit.
fn decimal(text: &str, precision: u8, scale: u8) -> Option<DecimalValue> {
    if !is_canonical_decimal(text, scale) {
        return None;
    }
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let integer = unsigned.split('.').next().unwrap_or_default();
    let integer_digits = if integer == "0" { 0 } else { integer.len() };
    if integer_digits > usize::from(precision.saturating_sub(scale)) {
        return None;
    }
    let digits: String = text.chars().filter(|character| *character != '.').collect();
    digits
        .parse::<i128>()
        .ok()
        .map(|unscaled| DecimalValue::new(unscaled, scale))
}

/// A canonical date `YYYY-MM-DD` that names a valid date.
fn date(text: &str) -> Option<DateValue> {
    if !is_canonical_date(text) {
        return None;
    }
    let mut parts = text.split('-');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    DateValue::new(year, month, day)
}

/// What a value of `declared` must be, in English.
fn expected(declared: ScalarType) -> String {
    match declared {
        ScalarType::Text => "a JSON string, as `text` requires".to_owned(),
        ScalarType::Integer => "a signed 64-bit `integer`: a JSON number within ±(2^53 − 1), \
             or outside that range a string of an optional `-` and digits without leading zeros"
            .to_owned(),
        ScalarType::Decimal { precision, scale } => format!(
            "a `decimal({precision},{scale})`: a canonical decimal string with exactly {scale} \
             fraction digits and at most {} integer digits",
            precision.saturating_sub(scale)
        ),
        ScalarType::Date => "a valid `date` written as the string `YYYY-MM-DD`".to_owned(),
        ScalarType::Boolean => "a JSON boolean, as `boolean` requires".to_owned(),
    }
}

/// The type as SCY-2.2 writes it.
fn name(declared: ScalarType) -> String {
    match declared {
        ScalarType::Text => "text".to_owned(),
        ScalarType::Integer => "integer".to_owned(),
        ScalarType::Decimal { precision, scale } => format!("decimal({precision},{scale})"),
        ScalarType::Date => "date".to_owned(),
        ScalarType::Boolean => "boolean".to_owned(),
    }
}

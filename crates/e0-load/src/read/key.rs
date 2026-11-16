//! When two mapping keys are the same key (E0A-2.3).
//!
//! Keys are compared after construction, the way the reference loader's mappings compare them:
//! `1`, `0x1`, `1.0` and `true` are one key; `1` and `'1'` are two. Dates and timestamps are
//! compared as their ISO 8601 text (E0A-2.8), so `2026-11-16` and `'2026-11-16'` are one key.
//! A `.nan` key equals no other key.

use super::tree::Value;

/// A scalar key reduced to what makes it equal to another key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum KeyIdentity {
    Null,
    /// A number with an integral value (booleans are 0 and 1), as canonical decimal text.
    Integral(String),
    /// A float with a fractional part, by its bits.
    Fractional(u64),
    PositiveInfinity,
    NegativeInfinity,
    Text(String),
}

/// The identity of a scalar key, or `None` when the key equals no other key (`.nan`) or is not
/// a scalar.
pub(crate) fn key_identity(value: &Value) -> Option<KeyIdentity> {
    match value {
        Value::Null => Some(KeyIdentity::Null),
        Value::Boolean(value) => Some(KeyIdentity::Integral(
            if *value { "1" } else { "0" }.to_owned(),
        )),
        Value::Integer(value) => Some(KeyIdentity::Integral(value.to_string())),
        Value::Float(value) => float_identity(*value),
        Value::String(text) => Some(KeyIdentity::Text(text.clone())),
        Value::Sequence(_) | Value::Mapping(_) => None,
    }
}

fn float_identity(value: f64) -> Option<KeyIdentity> {
    if value.is_nan() {
        None
    } else if value == f64::INFINITY {
        Some(KeyIdentity::PositiveInfinity)
    } else if value == f64::NEG_INFINITY {
        Some(KeyIdentity::NegativeInfinity)
    } else if value == 0.0 {
        Some(KeyIdentity::Integral("0".to_owned()))
    } else if value.fract() == 0.0 {
        // With a precision, the exact decimal value is printed, so it compares with integers.
        Some(KeyIdentity::Integral(format!("{value:.0}")))
    } else {
        Some(KeyIdentity::Fractional(value.to_bits()))
    }
}

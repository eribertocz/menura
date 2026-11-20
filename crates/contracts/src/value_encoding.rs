//! CON-4 value encoding: which values are JSON strings in canonical form, and those forms.
//!
//! RFC 8785 writes numbers as IEEE-754 doubles, so every value whose exact value matters is a
//! string: integers outside ±(2^53 − 1), decimals, dates, times, timestamps and UUIDs.

use serde_json::Value;

use crate::schema::strings::is_full_date;

/// The largest integer every IEEE-754 double represents exactly: 2^53 − 1.
pub const MAX_SAFE_INTEGER: i128 = 9_007_199_254_740_991;

/// Encodes an integer: a JSON number within ±(2^53 − 1), otherwise its decimal string.
///
/// ```
/// use menura_contracts::encode_integer;
///
/// assert_eq!(encode_integer(42), serde_json::json!(42));
/// assert_eq!(encode_integer(9_007_199_254_740_993), serde_json::json!("9007199254740993"));
/// ```
#[must_use]
pub fn encode_integer(value: i128) -> Value {
    match i64::try_from(value) {
        Ok(small) if value.abs() <= MAX_SAFE_INTEGER => Value::from(small),
        _ => Value::String(value.to_string()),
    }
}

/// Whether `text` is the canonical string of an integer outside ±(2^53 − 1): an optional minus
/// and digits without leading zeros. Integers within the range are JSON numbers instead.
#[must_use]
pub fn is_canonical_integer_string(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    let limit = MAX_SAFE_INTEGER.to_string();
    is_unpadded_digits(digits)
        && (digits.len() > limit.len() || (digits.len() == limit.len() && digits > limit.as_str()))
}

/// Whether `text` is a canonical decimal of `scale`: an optional minus, the integer digits
/// without leading zeros, then `.` and exactly `scale` digits. With `scale` 0 there is no `.`.
/// Zero has no minus sign, so each value has one form.
#[must_use]
pub fn is_canonical_decimal(text: &str, scale: u8) -> bool {
    let (negative, unsigned) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (integer, fraction) = match unsigned.split_once('.') {
        Some((integer, fraction)) if scale > 0 => (integer, fraction),
        None if scale == 0 => (unsigned, ""),
        _ => return false,
    };
    let is_zero = integer == "0" && fraction.bytes().all(|byte| byte == b'0');
    is_unpadded_digits(integer)
        && fraction.len() == usize::from(scale)
        && fraction.bytes().all(|byte| byte.is_ascii_digit())
        && !(negative && is_zero)
}

/// Whether `text` is a canonical date, `YYYY-MM-DD`.
#[must_use]
pub fn is_canonical_date(text: &str) -> bool {
    is_full_date(text)
}

/// Whether `text` is a canonical time: `HH:MM:SS`, plus `.` and 1–9 fraction digits only when
/// the fraction is not zero, with trailing zeros removed.
#[must_use]
pub fn is_canonical_time(text: &str) -> bool {
    let (whole, fraction) = match text.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (text, None),
    };
    let parts: Vec<&str> = whole.split(':').collect();
    let [hour, minute, second] = parts.as_slice() else {
        return false;
    };
    is_two_digits_at_most(hour, 23)
        && is_two_digits_at_most(minute, 59)
        && is_two_digits_at_most(second, 59)
        && fraction.is_none_or(is_canonical_fraction)
}

/// Whether `text` is a canonical timestamp: `YYYY-MM-DDTHH:MM:SS` with the time fraction rule,
/// suffixed `Z` (UTC) exactly when the type is `with_time_zone`.
#[must_use]
pub fn is_canonical_timestamp(text: &str, with_time_zone: bool) -> bool {
    let local = if with_time_zone {
        text.strip_suffix('Z')
    } else {
        Some(text)
    };
    let Some((date, time)) = local.and_then(|local| local.split_once('T')) else {
        return false;
    };
    is_canonical_date(date) && is_canonical_time(time)
}

/// Whether `text` is a canonical UUID: lowercase hexadecimal, hyphenated 8-4-4-4-12.
#[must_use]
pub fn is_canonical_uuid(text: &str) -> bool {
    let groups: Vec<&str> = text.split('-').collect();
    groups.len() == 5
        && groups.iter().zip([8, 4, 4, 4, 12]).all(|(group, length)| {
            group.len() == length
                && group
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
}

fn is_canonical_fraction(fraction: &str) -> bool {
    (1..=9).contains(&fraction.len())
        && fraction.bytes().all(|byte| byte.is_ascii_digit())
        && !fraction.ends_with('0')
}

fn is_unpadded_digits(text: &str) -> bool {
    !text.is_empty()
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'))
}

fn is_two_digits_at_most(text: &str, maximum: u32) -> bool {
    text.len() == 2
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && text.parse::<u32>().is_ok_and(|value| value <= maximum)
}

//! Construction of YAML 1.1 booleans, integers and floats, as the reference loader constructs
//! them (E0A-2.2, E0A-2.5).
//!
//! Underscores are ignored. Integers take the base of their prefix (`0b` binary, `0x`
//! hexadecimal, a leading `0` octal) or are sexagesimal (`1:30` is 90); floats may be sexagesimal
//! too (`1:30.5` is 90.5).

use super::tree::Integer;

/// Constructs a `!!bool` value; the case of the letters does not matter.
pub(crate) fn construct_boolean(text: &str) -> Option<bool> {
    match text.to_lowercase().as_str() {
        "yes" | "true" | "on" => Some(true),
        "no" | "false" | "off" => Some(false),
        _ => None,
    }
}

/// Constructs an `!!int` value, of any size.
pub(crate) fn construct_integer(text: &str) -> Option<Integer> {
    let cleaned = text.replace('_', "");
    let (negative, body) = split_sign(&cleaned);
    let magnitude = if body == "0" {
        Magnitude::zero()
    } else if let Some(digits) = body.strip_prefix("0b") {
        Magnitude::parse(digits, 2)?
    } else if let Some(digits) = body.strip_prefix("0x") {
        Magnitude::parse(digits, 16)?
    } else if body.starts_with('0') {
        Magnitude::parse(body, 8)?
    } else if body.contains(':') {
        sexagesimal_integer(body)?
    } else {
        Magnitude::parse(body, 10)?
    };
    Some(Integer::new(negative, magnitude.decimal()))
}

/// Constructs a `!!float` value.
pub(crate) fn construct_float(text: &str) -> Option<f64> {
    let cleaned = text.replace('_', "").to_lowercase();
    let (negative, body) = split_sign(&cleaned);
    let magnitude = match body {
        ".inf" => f64::INFINITY,
        ".nan" => return Some(f64::NAN),
        _ if body.contains(':') => sexagesimal_float(body)?,
        _ => body.parse::<f64>().ok()?,
    };
    Some(if negative { -magnitude } else { magnitude })
}

/// Splits one leading `+` or `-` from the text.
fn split_sign(text: &str) -> (bool, &str) {
    if let Some(rest) = text.strip_prefix('-') {
        (true, rest)
    } else {
        (false, text.strip_prefix('+').unwrap_or(text))
    }
}

/// `a:b:c` is `a * 60 * 60 + b * 60 + c`.
fn sexagesimal_integer(body: &str) -> Option<Magnitude> {
    let mut value = Magnitude::zero();
    for part in body.split(':') {
        value.multiply_add(60, 0);
        value.add(&Magnitude::parse(part, 10)?);
    }
    Some(value)
}

/// `a:b:c.d` is `a * 60 * 60 + b * 60 + c.d`, summed from the last part as the reference does.
fn sexagesimal_float(body: &str) -> Option<f64> {
    let mut value = 0.0;
    let mut weight = 1.0;
    for part in body.rsplit(':') {
        value += part.parse::<f64>().ok()? * weight;
        weight *= 60.0;
    }
    Some(value)
}

/// A natural number of any size, as decimal digits, least significant first.
struct Magnitude {
    digits: Vec<u8>,
}

impl Magnitude {
    fn zero() -> Self {
        Self { digits: vec![0] }
    }

    /// Parses at least one digit in `radix`; `None` when a character is not such a digit.
    fn parse(text: &str, radix: u32) -> Option<Self> {
        if text.is_empty() {
            return None;
        }
        let mut value = Self::zero();
        for character in text.chars() {
            value.multiply_add(radix, character.to_digit(radix)?);
        }
        Some(value)
    }

    /// `self = self * factor + addend`, with `factor` and `addend` at most 60.
    fn multiply_add(&mut self, factor: u32, addend: u32) {
        let mut carry = addend;
        for digit in &mut self.digits {
            let product = u32::from(*digit) * factor + carry;
            *digit = decimal_digit(product);
            carry = product / 10;
        }
        while carry > 0 {
            self.digits.push(decimal_digit(carry));
            carry /= 10;
        }
    }

    /// `self = self + other`.
    fn add(&mut self, other: &Self) {
        let mut carry = 0;
        for index in 0..self.digits.len().max(other.digits.len()) {
            let left = self.digits.get(index).copied().map_or(0, u32::from);
            let right = other.digits.get(index).copied().map_or(0, u32::from);
            let sum = left + right + carry;
            let digit = decimal_digit(sum);
            match self.digits.get_mut(index) {
                Some(slot) => *slot = digit,
                None => self.digits.push(digit),
            }
            carry = sum / 10;
        }
        if carry > 0 {
            self.digits.push(decimal_digit(carry));
        }
    }

    /// The canonical decimal text: most significant digit first, without leading zeros.
    fn decimal(&self) -> String {
        let text: String = self
            .digits
            .iter()
            .rev()
            .map(|digit| char::from(b'0' + digit))
            .collect();
        let trimmed = text.trim_start_matches('0');
        if trimmed.is_empty() {
            "0".to_owned()
        } else {
            trimmed.to_owned()
        }
    }
}

/// The last decimal digit of `value`.
fn decimal_digit(value: u32) -> u8 {
    // `value % 10` is below 10, so it always fits.
    u8::try_from(value % 10).unwrap_or(0)
}

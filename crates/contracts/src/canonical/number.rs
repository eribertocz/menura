//! Serialization of a JSON number as RFC 8785 requires: the ECMAScript `Number.prototype.toString`
//! form of the IEEE-754 double closest to it.

/// Largest decimal exponent written without scientific notation (ECMA-262 §6.1.6.1.20).
const LARGEST_PLAIN_EXPONENT: i32 = 21;

/// Decimal exponents above this one are written without scientific notation (ECMA-262 §6.1.6.1.20).
const SMALLEST_PLAIN_EXPONENT: i32 = -6;

/// Shortest decimal digits of a positive finite double, with the decimal exponent `n` such that
/// the value is `0.d1 d2 … dk × 10^n`.
struct ShortestDigits {
    digits: String,
    exponent: i32,
}

/// Writes `value` in the ECMAScript form. Both zeros are written `0`.
///
/// RFC 8785 rejects NaN and infinities; `serde_json` values never hold them, so the caller never
/// passes one. They are still written deterministically (`NaN`, `Infinity`) rather than panicking.
pub(crate) fn write_number(value: f64, output: &mut String) {
    if value == 0.0 {
        output.push('0');
        return;
    }
    if value.is_nan() {
        output.push_str("NaN");
        return;
    }
    if value < 0.0 {
        output.push('-');
    }
    if value.is_infinite() {
        output.push_str("Infinity");
        return;
    }
    let shortest = shortest_digits(value.abs());
    write_digits(&shortest, output);
}

/// Finds the digits ECMA-262 requires: the fewest digits that parse back to the same double,
/// and among those the value closest to it, ties going to the even digit.
///
/// Rust's `LowerExp` form without a precision gives the fewest digits `k`; formatting again with
/// `k` significant digits rounds the exact binary value to the nearest, ties to even.
fn shortest_digits(magnitude: f64) -> ShortestDigits {
    let digit_count = parse_scientific(&format!("{magnitude:e}")).digits.len();
    let precision = digit_count.saturating_sub(1);
    parse_scientific(&format!("{magnitude:.precision$e}"))
}

/// Reads `d.ddde±x` into its digits and the exponent `n` of `0.ddd × 10^n`.
fn parse_scientific(scientific: &str) -> ShortestDigits {
    let (mantissa, exponent_text) = scientific.split_once('e').unwrap_or((scientific, "0"));
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let exponent = exponent_text.parse::<i32>().unwrap_or(0) + 1;
    ShortestDigits { digits, exponent }
}

/// Applies the layout rules of ECMA-262 `Number::toString` to the digits.
fn write_digits(shortest: &ShortestDigits, output: &mut String) {
    let digits = shortest.digits.as_str();
    let digit_count = i32::try_from(digits.len()).unwrap_or(i32::MAX);
    let exponent = shortest.exponent;
    if digit_count <= exponent && exponent <= LARGEST_PLAIN_EXPONENT {
        output.push_str(digits);
        push_zeros(exponent - digit_count, output);
    } else if 0 < exponent && exponent <= LARGEST_PLAIN_EXPONENT {
        let (integer, fraction) = digits.split_at(usize::try_from(exponent).unwrap_or(0));
        output.push_str(integer);
        output.push('.');
        output.push_str(fraction);
    } else if SMALLEST_PLAIN_EXPONENT < exponent && exponent <= 0 {
        output.push_str("0.");
        push_zeros(-exponent, output);
        output.push_str(digits);
    } else {
        write_scientific(digits, exponent - 1, output);
    }
}

/// Writes `d[.ddd]e±x`.
fn write_scientific(digits: &str, exponent: i32, output: &mut String) {
    let (first, rest) = digits.split_at(1);
    output.push_str(first);
    if !rest.is_empty() {
        output.push('.');
        output.push_str(rest);
    }
    output.push('e');
    output.push(if exponent < 0 { '-' } else { '+' });
    output.push_str(&exponent.unsigned_abs().to_string());
}

fn push_zeros(count: i32, output: &mut String) {
    for _ in 0..count {
        output.push('0');
    }
}

#[cfg(test)]
mod tests {
    use super::write_number;

    fn written(bits: u64) -> String {
        let mut output = String::new();
        write_number(f64::from_bits(bits), &mut output);
        output
    }

    /// RFC 8785 Appendix B: IEEE-754 bit patterns and their canonical text.
    #[test]
    fn rfc_8785_appendix_b_numbers() {
        let vectors: [(u64, &str); 24] = [
            (0x0000_0000_0000_0000, "0"),
            (0x8000_0000_0000_0000, "0"),
            (0x0000_0000_0000_0001, "5e-324"),
            (0x8000_0000_0000_0001, "-5e-324"),
            (0x7fef_ffff_ffff_ffff, "1.7976931348623157e+308"),
            (0xffef_ffff_ffff_ffff, "-1.7976931348623157e+308"),
            (0x4340_0000_0000_0000, "9007199254740992"),
            (0xc340_0000_0000_0000, "-9007199254740992"),
            (0x4430_0000_0000_0000, "295147905179352830000"),
            (0x44b5_2d02_c7e1_4af5, "9.999999999999997e+22"),
            (0x44b5_2d02_c7e1_4af6, "1e+23"),
            (0x44b5_2d02_c7e1_4af7, "1.0000000000000001e+23"),
            (0x444b_1ae4_d6e2_ef4e, "999999999999999700000"),
            (0x444b_1ae4_d6e2_ef4f, "999999999999999900000"),
            (0x444b_1ae4_d6e2_ef50, "1e+21"),
            (0x3eb0_c6f7_a0b5_ed8c, "9.999999999999997e-7"),
            (0x3eb0_c6f7_a0b5_ed8d, "0.000001"),
            (0x41b3_de43_5555_5553, "333333333.3333332"),
            (0x41b3_de43_5555_5554, "333333333.33333325"),
            (0x41b3_de43_5555_5555, "333333333.3333333"),
            (0x41b3_de43_5555_5556, "333333333.3333334"),
            (0x41b3_de43_5555_5557, "333333333.33333343"),
            (0xbecb_f647_612f_3696, "-0.0000033333333333333333"),
            (0x4314_3ff3_c1cb_0959, "1424953923781206.2"),
        ];
        for (bits, expected) in vectors {
            assert_eq!(written(bits), expected, "bits {bits:#018x}");
        }
    }
}

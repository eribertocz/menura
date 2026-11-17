//! Result types of arithmetic over numeric types (E0F-5a).

use semantic_compiler_contracts::types::common::LogicalType;

use super::diagnostic::TypingCode;
use super::stop::{Outcome, problem};
use super::value_type::{Numeric, decimal, double, huge_integer};

/// The largest decimal precision and scale (E0F-5a).
const MAXIMUM_DIGITS: u8 = 38;

/// The arithmetic operations of E0F-5a, by result rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Arithmetic {
    /// `+`, `-`, `%` and `MOD`.
    AddSubtractModulo,
    /// `*`.
    Multiply,
    /// `/`, true division (E0F-2.5).
    Divide,
}

/// The result type of `left operation right`.
///
/// # Errors
///
/// `E0F-08` when a decimal product has a scale over 38.
pub(crate) fn arithmetic(
    operation: Arithmetic,
    left: Numeric,
    right: Numeric,
) -> Outcome<LogicalType> {
    if operation == Arithmetic::Divide || left == Numeric::Float || right == Numeric::Float {
        return Ok(double());
    }
    match (operation, left, right) {
        (_, Numeric::Integer { .. }, Numeric::Integer { .. }) => Ok(huge_integer()),
        (Arithmetic::Multiply, _, _) => product(as_decimal(left), as_decimal(right)),
        _ => Ok(sum(as_decimal(left), as_decimal(right))),
    }
}

/// The digits of a decimal operand of `+`, `-`, `%` and `*`: an integer counts as
/// `decimal(38,0)`.
fn as_decimal(operand: Numeric) -> (u8, u8) {
    match operand {
        Numeric::Decimal { precision, scale } => (precision, scale),
        Numeric::Integer { .. } | Numeric::Float => (MAXIMUM_DIGITS, 0),
    }
}

/// `decimal(min(38, max(p1−s1, p2−s2) + max(s1,s2) + 1), max(s1,s2))`.
fn sum(
    (left_precision, left_scale): (u8, u8),
    (right_precision, right_scale): (u8, u8),
) -> LogicalType {
    let integer_digits = left_precision
        .saturating_sub(left_scale)
        .max(right_precision.saturating_sub(right_scale));
    let scale = left_scale.max(right_scale);
    let precision = (u16::from(integer_digits) + u16::from(scale) + 1).min(MAXIMUM_DIGITS.into());
    decimal(u8::try_from(precision).unwrap_or(MAXIMUM_DIGITS), scale)
}

/// `decimal(min(38, p1+p2), s1+s2)`; `s1+s2 > 38` is `E0F-08`.
fn product(
    (left_precision, left_scale): (u8, u8),
    (right_precision, right_scale): (u8, u8),
) -> Outcome<LogicalType> {
    let scale = u16::from(left_scale) + u16::from(right_scale);
    if scale > MAXIMUM_DIGITS.into() {
        return problem(
            TypingCode::DecimalScaleOverflow,
            format!(
                "the product of decimal({left_precision},{left_scale}) and \
                 decimal({right_precision},{right_scale}) has scale {scale}, over 38"
            ),
        );
    }
    let precision =
        (u16::from(left_precision) + u16::from(right_precision)).min(MAXIMUM_DIGITS.into());
    Ok(decimal(
        u8::try_from(precision).unwrap_or(MAXIMUM_DIGITS),
        u8::try_from(scale).unwrap_or(MAXIMUM_DIGITS),
    ))
}

/// The type of `-x` and `ABS(x)` for a numeric `x` of type `operand`: the operand type, with
/// integers widened to 128 bits.
pub(crate) fn widened(operand: Numeric, operand_type: &LogicalType) -> LogicalType {
    match operand {
        Numeric::Integer { .. } => huge_integer(),
        Numeric::Decimal { .. } | Numeric::Float => operand_type.clone(),
    }
}

/// The type of `FLOOR(x)`, `CEIL(x)` and `CEILING(x)` (E0F-10.3).
pub(crate) fn floor(operand: Numeric) -> LogicalType {
    match operand {
        Numeric::Integer { .. } => huge_integer(),
        Numeric::Decimal { precision, scale } => {
            decimal((precision.saturating_sub(scale) + 1).min(MAXIMUM_DIGITS), 0)
        }
        Numeric::Float => double(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Arithmetic, arithmetic, floor};
    use crate::typing::stop::Stop;
    use crate::typing::value_type::{Numeric, decimal, double, huge_integer};

    const INTEGER: Numeric = Numeric::Integer {
        bits: 32,
        signed: true,
    };

    fn number(precision: u8, scale: u8) -> Numeric {
        Numeric::Decimal { precision, scale }
    }

    #[test]
    fn e0f_5a_integer_arithmetic_widens_and_division_is_float() {
        for operation in [Arithmetic::AddSubtractModulo, Arithmetic::Multiply] {
            assert_eq!(arithmetic(operation, INTEGER, INTEGER), Ok(huge_integer()));
        }
        assert_eq!(
            arithmetic(Arithmetic::Divide, INTEGER, INTEGER),
            Ok(double())
        );
        assert_eq!(
            arithmetic(Arithmetic::Divide, number(10, 2), number(10, 2)),
            Ok(double())
        );
        assert_eq!(
            arithmetic(Arithmetic::AddSubtractModulo, Numeric::Float, INTEGER),
            Ok(double())
        );
    }

    #[test]
    fn e0f_5a_decimal_sums_and_products() {
        assert_eq!(
            arithmetic(Arithmetic::AddSubtractModulo, number(10, 2), number(5, 4)),
            Ok(decimal(13, 4))
        );
        assert_eq!(
            arithmetic(Arithmetic::AddSubtractModulo, number(10, 2), INTEGER),
            Ok(decimal(38, 2))
        );
        assert_eq!(
            arithmetic(Arithmetic::Multiply, number(10, 2), number(5, 4)),
            Ok(decimal(15, 6))
        );
        assert_eq!(
            arithmetic(Arithmetic::Multiply, number(30, 20), number(20, 10)),
            Ok(decimal(38, 30))
        );
        assert_eq!(
            arithmetic(Arithmetic::Multiply, number(10, 2), INTEGER),
            Ok(decimal(38, 2))
        );
    }

    #[test]
    fn e0f_08_a_decimal_product_over_scale_38() {
        let Err(Stop::Problem { code, .. }) =
            arithmetic(Arithmetic::Multiply, number(30, 20), number(30, 19))
        else {
            panic!("the product must fail");
        };
        assert_eq!(code.as_str(), "E0F-08");
    }

    #[test]
    fn e0f_10_3_floor_types() {
        assert_eq!(floor(INTEGER), huge_integer());
        assert_eq!(floor(number(10, 2)), decimal(9, 0));
        assert_eq!(floor(number(38, 0)), decimal(38, 0));
        assert_eq!(floor(Numeric::Float), double());
    }
}

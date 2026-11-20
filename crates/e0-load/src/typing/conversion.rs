//! Implicit conversions (E0F-3) and the CASE rule of E0F-5a: the type to which every other
//! type of a list converts implicitly.

use menura_contracts::types::common::LogicalType;

use super::value_type::{ExpressionType, Numeric, numeric};

/// Whether a value of type `from` converts implicitly to `to` (E0F-3): only when no
/// information can be lost. The literal `NULL` converts to every type.
pub(crate) fn converts(from: &ExpressionType, to: &ExpressionType) -> bool {
    match (from, to) {
        (ExpressionType::Null, _) => true,
        (ExpressionType::Value(_), ExpressionType::Null) => false,
        (ExpressionType::Value(from), ExpressionType::Value(to)) => converts_value(from, to),
    }
}

fn converts_value(from: &LogicalType, to: &LogicalType) -> bool {
    if from == to {
        return true;
    }
    match (numeric(from), numeric(to)) {
        (Some(from), Some(to)) => converts_numeric(from, to),
        _ => false,
    }
}

fn converts_numeric(from: Numeric, to: Numeric) -> bool {
    match (from, to) {
        (
            Numeric::Integer { bits, signed },
            Numeric::Integer {
                bits: to_bits,
                signed: to_signed,
            },
        ) => integer_fits(bits, signed, to_bits, to_signed),
        (Numeric::Integer { bits, signed }, Numeric::Decimal { precision, scale }) => {
            integer_digits(bits, signed) <= u16::from(precision.saturating_sub(scale))
        }
        (
            Numeric::Decimal { precision, scale },
            Numeric::Decimal {
                precision: to_precision,
                scale: to_scale,
            },
        ) => {
            to_scale >= scale
                && to_precision.saturating_sub(to_scale) >= precision.saturating_sub(scale)
        }
        _ => false,
    }
}

/// Whether every value of an integer of width `bits` and signedness `signed` is a value of the
/// integer of width `to_bits` and signedness `to_signed`.
fn integer_fits(bits: u16, signed: bool, to_bits: u16, to_signed: bool) -> bool {
    match (signed, to_signed) {
        (true, false) => false,
        (false, true) => to_bits > bits,
        _ => to_bits >= bits,
    }
}

/// The decimal digits of the largest magnitude of an integer type: the integer digits a
/// decimal needs to hold all its values.
fn integer_digits(bits: u16, signed: bool) -> u16 {
    match (bits, signed) {
        (8, _) => 3,
        (16, _) => 5,
        (32, _) => 10,
        (64, true) => 19,
        (64, false) => 20,
        _ => 39,
    }
}

/// The CASE rule (E0F-5a): the first of `types` to which every other converts implicitly, or
/// `None` when there is none.
pub(crate) fn common_type(types: &[&ExpressionType]) -> Option<ExpressionType> {
    types
        .iter()
        .find(|candidate| types.iter().all(|other| converts(other, candidate)))
        .map(|found| (*found).clone())
}

#[cfg(test)]
mod tests {
    use menura_contracts::types::common::{IntegerBits, LogicalType};

    use super::{common_type, converts};
    use crate::typing::value_type::{ExpressionType, big_integer, decimal, double, integer};

    fn value(value_type: LogicalType) -> ExpressionType {
        ExpressionType::Value(value_type)
    }

    fn unsigned(bits: IntegerBits) -> ExpressionType {
        value(LogicalType::Integer {
            bits,
            signed: false,
        })
    }

    #[test]
    fn e0f_3_only_lossless_conversions_are_implicit() {
        let yes = |from: ExpressionType, to: ExpressionType| assert!(converts(&from, &to));
        let no = |from: ExpressionType, to: ExpressionType| assert!(!converts(&from, &to));
        yes(value(integer()), value(big_integer()));
        no(value(big_integer()), value(integer()));
        yes(unsigned(IntegerBits::ThirtyTwo), value(big_integer()));
        no(unsigned(IntegerBits::SixtyFour), value(big_integer()));
        no(value(integer()), unsigned(IntegerBits::SixtyFour));
        yes(value(integer()), value(decimal(12, 2)));
        no(value(integer()), value(decimal(11, 2)));
        no(value(big_integer()), value(decimal(38, 20)));
        yes(value(decimal(10, 2)), value(decimal(12, 4)));
        no(value(decimal(10, 2)), value(decimal(12, 1)));
        yes(value(decimal(10, 2)), value(decimal(11, 3)));
        no(value(decimal(10, 2)), value(decimal(10, 3)));
        no(value(decimal(10, 0)), value(big_integer()));
        no(value(integer()), value(double()));
        no(value(decimal(10, 2)), value(double()));
        no(
            value(LogicalType::Date),
            value(super::super::value_type::timestamp()),
        );
        no(value(integer()), value(LogicalType::Text));
        no(value(LogicalType::Text), value(LogicalType::Date));
        yes(ExpressionType::Null, value(LogicalType::Text));
        no(value(LogicalType::Text), ExpressionType::Null);
    }

    #[test]
    fn e0f_5a_the_case_rule_picks_the_type_every_other_converts_to() {
        let int = value(integer());
        let big = value(big_integer());
        let text = value(LogicalType::Text);
        assert_eq!(common_type(&[&int, &big, &ExpressionType::Null]), Some(big));
        assert_eq!(common_type(&[&int, &text]), None);
        assert_eq!(
            common_type(&[&ExpressionType::Null, &ExpressionType::Null]),
            Some(ExpressionType::Null)
        );
    }
}

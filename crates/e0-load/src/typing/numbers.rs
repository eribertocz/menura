//! Types the number functions of E0F-10.3, with the result types of E0F-5a.

use menura_contracts::types::common::LogicalType;
use menura_model::expression::{Expression, Literal, ScalarFunction, UnaryOperator};

use super::arithmetic::{Arithmetic, floor, widened};
use super::operators::arithmetic_type;
use super::stop::{Outcome, Stop, incompatible, untyped_null};
use super::typer::{any_nullable, expect_numeric};
use super::value_type::{Numeric, Typed, big_integer, decimal, double, huge_integer};

/// The type of a number function of E0F-10.3, or of `ZEROIFNULL` and `NULLIFZERO`.
pub(super) fn number_function(
    function: ScalarFunction,
    arguments: &[Expression],
    typed: &[Typed],
) -> Outcome<Typed> {
    match function {
        ScalarFunction::Round | ScalarFunction::Truncate => round(arguments, typed),
        ScalarFunction::Modulo => Ok(Typed::value(
            arithmetic_type(Arithmetic::AddSubtractModulo, typed)?,
            true,
        )),
        ScalarFunction::Sign => {
            numeric_arguments(typed)?;
            Ok(Typed::value(big_integer(), any_nullable(typed)))
        }
        ScalarFunction::Absolute
        | ScalarFunction::Floor
        | ScalarFunction::Ceiling
        | ScalarFunction::ZeroIfNull
        | ScalarFunction::NullIfZero => same_kind(function, typed),
        // `POWER`, `SQRT`, `EXP`, `LN`, `LOG`, `LOG10`: an argument outside the domain gives
        // NULL.
        _ => {
            numeric_arguments(typed)?;
            Ok(Typed::value(double(), true))
        }
    }
}

fn numeric_arguments(typed: &[Typed]) -> Outcome<Vec<Option<Numeric>>> {
    typed
        .iter()
        .map(|argument| expect_numeric(argument, "an argument of a number function"))
        .collect()
}

/// The single numeric argument of `typed` with its type; `E0F-03` for the literal `NULL`, which
/// no other argument gives a type (E0F-3.2).
fn single_number(typed: &[Typed]) -> Outcome<(Numeric, &LogicalType, bool)> {
    let [argument] = typed else {
        return Err(Stop::ReportedElsewhere);
    };
    let role = "the argument of a number function";
    let kind = expect_numeric(argument, role)?;
    match (kind, argument.logical()) {
        (Some(kind), Some(value_type)) => Ok((kind, value_type, argument.nullable)),
        _ => untyped_null(role),
    }
}

/// `ABS`, `FLOOR`, `CEIL`, `ZEROIFNULL` and `NULLIFZERO`, whose type follows the argument's.
fn same_kind(function: ScalarFunction, typed: &[Typed]) -> Outcome<Typed> {
    let (kind, value_type, nullable) = single_number(typed)?;
    Ok(match function {
        ScalarFunction::Absolute => Typed::value(widened(kind, value_type), nullable),
        ScalarFunction::Floor | ScalarFunction::Ceiling => Typed::value(floor(kind), nullable),
        ScalarFunction::ZeroIfNull => Typed::value(value_type.clone(), false),
        _ => Typed::value(value_type.clone(), true),
    })
}

/// `ROUND(x[, n])` and `TRUNC(x[, n])` (E0F-5a): one argument means `n = 0`.
fn round(arguments: &[Expression], typed: &[Typed]) -> Outcome<Typed> {
    let (operand, digits) = match (typed, arguments) {
        ([operand], _) => (operand, 0),
        ([operand, places], [_, written]) => {
            expect_numeric(places, "the digits of ROUND and TRUNC")?;
            let Some(digits) = integer_literal(written) else {
                return incompatible(
                    "the digits of ROUND and TRUNC must be an integer literal".to_owned(),
                );
            };
            (operand, digits)
        }
        _ => return Err(Stop::ReportedElsewhere),
    };
    let (kind, _, nullable) = single_number(std::slice::from_ref(operand))?;
    let value_type = match kind {
        Numeric::Float => double(),
        Numeric::Decimal { precision, scale } => match u8::try_from(digits) {
            Ok(digits) if digits <= scale => decimal(precision, digits),
            _ => {
                return incompatible(format!(
                    "ROUND and TRUNC of decimal({precision},{scale}) need from 0 to {scale} digits, not {digits}"
                ));
            }
        },
        Numeric::Integer { .. } if digits <= 0 => huge_integer(),
        Numeric::Integer { .. } => {
            return incompatible(format!(
                "ROUND and TRUNC of an integer need at most 0 digits, not {digits}"
            ));
        }
    };
    Ok(Typed::value(value_type, nullable))
}

/// The value of an integer literal, possibly negated.
fn integer_literal(expression: &Expression) -> Option<i64> {
    match expression {
        Expression::Literal(Literal::Integer(number)) => number.digits().parse().ok(),
        Expression::Unary {
            operator: UnaryOperator::Minus,
            operand,
        } => integer_literal(operand).map(|value| -value),
        _ => None,
    }
}

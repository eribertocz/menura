//! The numeric functions (E0F-10.3) and `%`: rounding half away from zero, truncation toward
//! zero, NULL outside a function's domain and for a zero divisor.

use menura_contracts::types::common::LogicalType;
use menura_model::expression::{Expression, ScalarFunction};

use super::ExpressionWriter;
use super::division::ZeroDivisor;
use crate::error::{GenerateError, InvariantViolation};

/// The digits of a 128-bit integer: rounding or truncating to `10^39` or more gives 0.
const HUGE_INTEGER_DIGITS: u32 = 38;

/// How `ROUND` and `TRUNC` treat the digits they drop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rounding {
    /// Half away from zero (E0F-2.5).
    Round,
    /// Toward zero.
    Truncate,
}

impl ExpressionWriter<'_, '_> {
    /// The call of a numeric `function`, before the cast to its result type `result`.
    pub(super) fn number_function(
        &mut self,
        expression: &Expression,
        function: ScalarFunction,
        arguments: &[Expression],
        result: &str,
    ) -> Result<String, GenerateError> {
        match function {
            ScalarFunction::Absolute => self.absolute(expression, arguments),
            ScalarFunction::Sign => self.unary_call("sign", arguments),
            ScalarFunction::Floor => self.unary_call("floor", arguments),
            ScalarFunction::Ceiling => self.unary_call("ceil", arguments),
            ScalarFunction::Round => self.rounded(Rounding::Round, arguments),
            ScalarFunction::Truncate => self.rounded(Rounding::Truncate, arguments),
            ScalarFunction::Modulo => {
                let [dividend, divisor] = self.exactly(arguments)?;
                self.modulo(expression, dividend, divisor, ZeroDivisor::Null)
            }
            ScalarFunction::Power => self.power(arguments),
            ScalarFunction::SquareRoot => self.guarded_double("sqrt", "< 0", arguments),
            ScalarFunction::Exponential => {
                let [value] = self.exactly(arguments)?;
                Ok(format!("exp({})", self.double(value)?))
            }
            ScalarFunction::NaturalLogarithm => self.guarded_double("ln", "<= 0", arguments),
            ScalarFunction::DecimalLogarithm => self.guarded_double("log10", "<= 0", arguments),
            ScalarFunction::Logarithm => self.logarithm(arguments),
            ScalarFunction::Greatest => self.extreme("greatest", arguments, result),
            ScalarFunction::Least => self.extreme("least", arguments, result),
            _ => Err(self.error(InvariantViolation::MalformedFunctionCall)),
        }
    }

    /// `x % y`, and `MOD(x, y)`: `zero` when `y = 0` (E6-4.6, E6-4.6a), NULL when `y` is NULL,
    /// the operands widened to `HUGEINT` when the result is a 128-bit integer (E0F-5a). The
    /// divisor is written first, where it first appears.
    pub(super) fn modulo(
        &mut self,
        expression: &Expression,
        dividend: &Expression,
        divisor: &Expression,
        zero: ZeroDivisor<'_>,
    ) -> Result<String, GenerateError> {
        let huge = self.is_huge_integer(expression);
        let divisor_text = self.operand(divisor)?;
        let dividend_text = self.operand(dividend)?;
        let remainder = if huge {
            format!("CAST({dividend_text} AS HUGEINT) % CAST({divisor_text} AS HUGEINT)")
        } else {
            format!("{dividend_text} % {divisor_text}")
        };
        let zero = zero.sql();
        Ok(format!(
            "CASE WHEN {divisor_text} = 0 THEN {zero} ELSE {remainder} END"
        ))
    }

    /// `abs(x)`, an integer widened to `HUGEINT` first so its absolute value fits.
    fn absolute(
        &mut self,
        expression: &Expression,
        arguments: &[Expression],
    ) -> Result<String, GenerateError> {
        let [value] = self.exactly(arguments)?;
        if self.is_huge_integer(expression) {
            return Ok(format!("abs({})", self.widened(value)?));
        }
        Ok(format!("abs({})", self.write(value)?))
    }

    /// `ROUND(x[, n])` or `TRUNC(x[, n])`, by the type of `x` (E0F-5a).
    fn rounded(
        &mut self,
        rounding: Rounding,
        arguments: &[Expression],
    ) -> Result<String, GenerateError> {
        let (value, digits) = match arguments {
            [value] => (value, 0),
            [value, digits] => (value, self.literal_integer(digits)?),
            _ => return Err(self.error(InvariantViolation::MalformedFunctionCall)),
        };
        match self.type_of(value)? {
            LogicalType::Integer { .. } => self.rounded_integer(rounding, value, digits),
            LogicalType::Decimal { scale, .. } => {
                self.rounded_decimal(rounding, value, digits, scale)
            }
            LogicalType::Float { .. } => self.rounded_float(rounding, value, digits),
            _ => Err(self.error(InvariantViolation::UntypedOperand)),
        }
    }

    /// An integer to `digits ≤ 0` digits, in 128-bit integer arithmetic, which holds every
    /// integer: truncated by subtracting its remainder, which has the sign of the integer, and
    /// rounded by adding one unit away from zero when the remainder is at least half a unit.
    fn rounded_integer(
        &mut self,
        rounding: Rounding,
        value: &Expression,
        digits: i64,
    ) -> Result<String, GenerateError> {
        let dropped = digits.unsigned_abs();
        if digits >= 0 {
            return self.widened(value);
        }
        if dropped > u64::from(HUGE_INTEGER_DIGITS) {
            return Ok(format!("{} * 0", self.widened(value)?));
        }
        let widened = self.widened(value)?;
        let unit = format!(
            "CAST('1{}' AS HUGEINT)",
            "0".repeat(usize::try_from(dropped).unwrap_or(0))
        );
        let remainder = format!("{widened} % {unit}");
        let truncated = format!("{widened} - {remainder}");
        match rounding {
            Rounding::Truncate => Ok(truncated),
            Rounding::Round => Ok(format!(
                "{truncated} + CASE WHEN abs({remainder}) >= {unit} - abs({remainder}) \
                 THEN sign({widened}) * {unit} ELSE 0 END"
            )),
        }
    }

    /// A `decimal(p,s)` to `0 ≤ digits ≤ s` digits. Truncation to `digits < s` moves `x` half
    /// a unit toward zero and rounds half away from zero, which is exact because `x` has more
    /// digits than the result, and never scales `x` up, so it cannot overflow.
    fn rounded_decimal(
        &mut self,
        rounding: Rounding,
        value: &Expression,
        digits: i64,
        scale: u8,
    ) -> Result<String, GenerateError> {
        let digits = u8::try_from(digits)
            .ok()
            .filter(|digits| *digits <= scale)
            .ok_or_else(|| self.error(InvariantViolation::MalformedFunctionCall))?;
        match rounding {
            Rounding::Round => Ok(format!("round({}, {digits})", self.write(value)?)),
            Rounding::Truncate if digits == scale => self.write(value),
            Rounding::Truncate => {
                let value = self.operand(value)?;
                let half = format!(
                    "CAST('0.{}5' AS DECIMAL({width},{width}))",
                    "0".repeat(usize::from(digits)),
                    width = digits + 1
                );
                Ok(format!("round({value} - sign({value}) * {half}, {digits})"))
            }
        }
    }

    /// A float to `digits` digits, in `DOUBLE` arithmetic, which no session setting changes.
    fn rounded_float(
        &mut self,
        rounding: Rounding,
        value: &Expression,
        digits: i64,
    ) -> Result<String, GenerateError> {
        if rounding == Rounding::Round {
            return Ok(format!("round({}, {digits})", self.write(value)?));
        }
        if digits == 0 {
            return Ok(format!("trunc({})", self.write(value)?));
        }
        let value = self.operand(value)?;
        let power = format!("1e{}", digits.unsigned_abs());
        if digits > 0 {
            Ok(format!("trunc({value} * {power}) / {power}"))
        } else {
            Ok(format!("trunc({value} / {power}) * {power}"))
        }
    }

    /// `x` cast to `DOUBLE`.
    fn double(&mut self, value: &Expression) -> Result<String, GenerateError> {
        Ok(format!("CAST({} AS DOUBLE)", self.write(value)?))
    }

    /// `CASE WHEN x <test> THEN NULL ELSE name(CAST(x AS DOUBLE)) END`: NULL outside the
    /// domain, where `DuckDB` raises an error.
    fn guarded_double(
        &mut self,
        name: &str,
        test: &str,
        arguments: &[Expression],
    ) -> Result<String, GenerateError> {
        let [value] = self.exactly(arguments)?;
        let value = self.operand(value)?;
        Ok(format!(
            "CASE WHEN {value} {test} THEN NULL ELSE {name}(CAST({value} AS DOUBLE)) END"
        ))
    }

    /// `LOG(base, x)` as `ln(x) / ln(base)`, NULL for a base that is not positive or is 1 and
    /// for an `x` that is not positive.
    fn logarithm(&mut self, arguments: &[Expression]) -> Result<String, GenerateError> {
        let [base, value] = self.exactly(arguments)?;
        let base = self.operand(base)?;
        let value = self.operand(value)?;
        Ok(format!(
            "CASE WHEN {base} <= 0 OR {base} = 1 OR {value} <= 0 THEN NULL \
             ELSE ln(CAST({value} AS DOUBLE)) / ln(CAST({base} AS DOUBLE)) END"
        ))
    }

    /// `pow(x, y)`, NULL for a negative `x` with a fractional `y` and for 0 to a negative `y`.
    fn power(&mut self, arguments: &[Expression]) -> Result<String, GenerateError> {
        let [base, exponent] = self.exactly(arguments)?;
        let base = self.operand(base)?;
        let exponent = self.operand(exponent)?;
        Ok(format!(
            "CASE WHEN ({base} < 0 AND {exponent} <> floor({exponent})) \
             OR ({base} = 0 AND {exponent} < 0) THEN NULL \
             ELSE pow(CAST({base} AS DOUBLE), CAST({exponent} AS DOUBLE)) END"
        ))
    }

    /// `greatest(…)` or `least(…)` of the arguments cast to `result`, NULL when any is NULL;
    /// `DuckDB` skips NULLs.
    fn extreme(
        &mut self,
        name: &str,
        arguments: &[Expression],
        result: &str,
    ) -> Result<String, GenerateError> {
        let written = arguments
            .iter()
            .map(|argument| self.operand(argument))
            .collect::<Result<Vec<_>, _>>()?;
        let tests: Vec<String> = written
            .iter()
            .map(|text| format!("{text} IS NULL"))
            .collect();
        let casts: Vec<String> = written
            .iter()
            .map(|text| format!("CAST({text} AS {result})"))
            .collect();
        Ok(format!(
            "CASE WHEN {} THEN NULL ELSE {name}({}) END",
            tests.join(" OR "),
            casts.join(", ")
        ))
    }
}

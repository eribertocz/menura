//! Scalar function calls of E0F-10. Each family writes its call; the result is then cast to the
//! type E0F-10 and E0F-5a fix, whatever `ClickHouse` infers (E6-4.10, E6-7.9).

use menura_contracts::types::common::LogicalType;
use menura_model::expression::{Expression, Literal, ScalarFunction, UnaryOperator};

use super::ExpressionWriter;
use crate::error::{GenerateError, InvariantViolation};
use crate::types::cast_sql;

impl ExpressionWriter<'_, '_> {
    /// `CAST(<call> AS <result type>)` for the call `expression` of `function`.
    pub(super) fn function(
        &mut self,
        expression: &Expression,
        function: ScalarFunction,
        arguments: &[Expression],
    ) -> Result<String, GenerateError> {
        let result_type = self.type_of(expression)?;
        let call = match family(function) {
            Family::Date => self.date_function(function, arguments)?,
            Family::Text => self.text_function(function, arguments)?,
            Family::Number => {
                self.number_function(expression, function, arguments, &result_type)?
            }
            Family::Conditional => self.conditional_function(function, arguments, &result_type)?,
        };
        self.cast_value(&call, &result_type)
    }

    /// `CAST(<sql> AS <type of expression>)`: the result type of E0F-10 enforced (E6-4.10).
    pub(super) fn typed(
        &self,
        expression: &Expression,
        sql: &str,
    ) -> Result<String, GenerateError> {
        self.cast_value(sql, &self.type_of(expression)?)
    }

    /// `value` cast to `logical_type` as E6-7.9 states: checked for an integer.
    pub(crate) fn cast_value(
        &self,
        value: &str,
        logical_type: &LogicalType,
    ) -> Result<String, GenerateError> {
        cast_sql(value, logical_type).ok_or_else(|| self.type_error(logical_type))
    }

    /// The `N` arguments of a call that takes exactly `N`.
    pub(super) fn exactly<'call, const N: usize>(
        &self,
        arguments: &'call [Expression],
    ) -> Result<&'call [Expression; N], GenerateError> {
        <&[Expression; N]>::try_from(arguments)
            .map_err(|_| self.error(InvariantViolation::MalformedFunctionCall))
    }

    /// The value of a literal integer argument, such as the `n` of `ROUND(x, n)`, with its
    /// optional sign.
    pub(super) fn literal_integer(&self, expression: &Expression) -> Result<i64, GenerateError> {
        let malformed = || self.error(InvariantViolation::MalformedFunctionCall);
        match expression {
            Expression::Literal(Literal::Integer(number)) => {
                number.digits().parse().map_err(|_| malformed())
            }
            Expression::Unary {
                operator: UnaryOperator::Minus,
                operand,
            } => self
                .literal_integer(operand)?
                .checked_neg()
                .ok_or_else(malformed),
            Expression::Unary {
                operator: UnaryOperator::Plus,
                operand,
            } => self.literal_integer(operand),
            _ => Err(malformed()),
        }
    }
}

/// The family of functions a call belongs to, each written by its own module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    /// E0F-10.1, `dates.rs`.
    Date,
    /// E0F-10.2, `text.rs`.
    Text,
    /// E0F-10.3, `numbers.rs`.
    Number,
    /// E0F-10.4, `conditionals.rs`.
    Conditional,
}

fn family(function: ScalarFunction) -> Family {
    match function {
        ScalarFunction::Year
        | ScalarFunction::Quarter
        | ScalarFunction::Month
        | ScalarFunction::Day
        | ScalarFunction::DayOfYear
        | ScalarFunction::Hour
        | ScalarFunction::Minute
        | ScalarFunction::Second
        | ScalarFunction::ToDate
        | ScalarFunction::ToTimestamp => Family::Date,
        ScalarFunction::Concat
        | ScalarFunction::Length
        | ScalarFunction::Lower
        | ScalarFunction::Upper
        | ScalarFunction::Trim
        | ScalarFunction::LeftTrim
        | ScalarFunction::RightTrim
        | ScalarFunction::Left
        | ScalarFunction::Right
        | ScalarFunction::Substring
        | ScalarFunction::Replace
        | ScalarFunction::SplitPart
        | ScalarFunction::Position
        | ScalarFunction::Contains
        | ScalarFunction::StartsWith
        | ScalarFunction::EndsWith => Family::Text,
        ScalarFunction::If
        | ScalarFunction::Nullif
        | ScalarFunction::Coalesce
        | ScalarFunction::IfNull
        | ScalarFunction::Nvl2
        | ScalarFunction::ZeroIfNull
        | ScalarFunction::NullIfZero => Family::Conditional,
        ScalarFunction::Absolute
        | ScalarFunction::Sign
        | ScalarFunction::Floor
        | ScalarFunction::Ceiling
        | ScalarFunction::Round
        | ScalarFunction::Truncate
        | ScalarFunction::Modulo
        | ScalarFunction::Power
        | ScalarFunction::SquareRoot
        | ScalarFunction::Exponential
        | ScalarFunction::NaturalLogarithm
        | ScalarFunction::Logarithm
        | ScalarFunction::DecimalLogarithm
        | ScalarFunction::Greatest
        | ScalarFunction::Least => Family::Number,
    }
}

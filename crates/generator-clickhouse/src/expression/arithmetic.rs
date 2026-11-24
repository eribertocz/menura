//! Arithmetic in the integer types of E0F-5a, checked so that it never wraps around (E6-7.9).

use menura_contracts::types::common::{IntegerBits, LogicalType};
use menura_model::expression::{BinaryOperator, Expression};

use super::ExpressionWriter;
use super::operators::operator_sql;
use crate::error::GenerateError;

impl ExpressionWriter<'_, '_> {
    /// `l op r`. When E0F-5a makes the result an integer, the operation is computed in
    /// `Int256`, which holds every result of two 128-bit operands, and the result is checked
    /// into its type, so that a value that does not fit raises an error instead of wrapping
    /// around (E6-7.9).
    pub(super) fn arithmetic(
        &mut self,
        expression: &Expression,
        operator: BinaryOperator,
        left: &Expression,
        right: &Expression,
    ) -> Result<String, GenerateError> {
        let symbol = operator_sql(operator);
        if let Some(result_type) = self.integer_result(expression) {
            let left = self.write(left)?;
            let right = self.write(right)?;
            let wide = format!("toInt256({left}) {symbol} toInt256({right})");
            return self.cast_value(&wide, &result_type);
        }
        let left = self.operand(left)?;
        let right = self.operand(right)?;
        Ok(format!("{left} {symbol} {right}"))
    }

    /// `-x`; an integer is negated in `Int256` and checked into its type (E6-7.9).
    pub(super) fn negation(
        &mut self,
        expression: &Expression,
        operand: &Expression,
    ) -> Result<String, GenerateError> {
        if let Some(result_type) = self.integer_result(expression) {
            let value = self.write(operand)?;
            return self.cast_value(&format!("-toInt256({value})"), &result_type);
        }
        Ok(format!("-{}", self.operand(operand)?))
    }

    /// The type of `expression` when it is an integer.
    fn integer_result(&self, expression: &Expression) -> Option<LogicalType> {
        self.known_type(expression)
            .filter(|logical_type| matches!(logical_type, LogicalType::Integer { .. }))
    }

    /// `operand` cast to `Int128`, unless it already is one.
    pub(super) fn widened(&mut self, operand: &Expression) -> Result<String, GenerateError> {
        if self.is_huge_integer(operand) {
            self.operand(operand)
        } else {
            Ok(format!("toInt128({})", self.write(operand)?))
        }
    }

    pub(super) fn is_huge_integer(&self, expression: &Expression) -> bool {
        self.known_type(expression)
            == Some(LogicalType::Integer {
                bits: IntegerBits::OneHundredTwentyEight,
                signed: true,
            })
    }
}

//! Integer arithmetic in the 128-bit integer of E0F-5a, whatever `ClickHouse` infers.

use menura_contracts::types::common::{IntegerBits, LogicalType};
use menura_model::expression::{BinaryOperator, Expression};

use super::ExpressionWriter;
use super::operators::operator_sql;
use crate::error::GenerateError;

impl ExpressionWriter<'_, '_> {
    /// `l op r`; when E0F-5a makes the result a 128-bit integer, both operands are cast to
    /// `Int128` first, so `ClickHouse` computes it without overflowing a narrower type.
    pub(super) fn arithmetic(
        &mut self,
        expression: &Expression,
        operator: BinaryOperator,
        left: &Expression,
        right: &Expression,
    ) -> Result<String, GenerateError> {
        let symbol = operator_sql(operator);
        if self.is_huge_integer(expression) {
            let left = self.widened(left)?;
            let right = self.widened(right)?;
            return Ok(format!("{left} {symbol} {right}"));
        }
        let left = self.operand(left)?;
        let right = self.operand(right)?;
        Ok(format!("{left} {symbol} {right}"))
    }

    /// `-x`; an integer is widened to `Int128` first (E0F-5a).
    pub(super) fn negation(
        &mut self,
        expression: &Expression,
        operand: &Expression,
    ) -> Result<String, GenerateError> {
        if self.is_huge_integer(expression) {
            return Ok(format!("-{}", self.widened(operand)?));
        }
        Ok(format!("-{}", self.operand(operand)?))
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

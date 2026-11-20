//! The conditional functions (E0F-10.4), each branch cast to the result type of the CASE rule
//! (E0F-5a).

use menura_model::expression::{Expression, ScalarFunction};

use super::ExpressionWriter;
use crate::error::{GenerateError, InvariantViolation};

impl ExpressionWriter<'_, '_> {
    /// The call of a conditional `function`, before the cast to its result type `result`.
    pub(super) fn conditional_function(
        &mut self,
        function: ScalarFunction,
        arguments: &[Expression],
        result: &str,
    ) -> Result<String, GenerateError> {
        match function {
            ScalarFunction::If => {
                let [condition, then, otherwise] = self.exactly(arguments)?;
                let condition = self.write(condition)?;
                let then = self.cast_branch(then, result)?;
                let otherwise = self.cast_branch(otherwise, result)?;
                Ok(format!(
                    "CASE WHEN {condition} THEN {then} ELSE {otherwise} END"
                ))
            }
            ScalarFunction::Nvl2 => {
                let [tested, present, absent] = self.exactly(arguments)?;
                let tested = self.operand(tested)?;
                let present = self.cast_branch(present, result)?;
                let absent = self.cast_branch(absent, result)?;
                Ok(format!(
                    "CASE WHEN {tested} IS NOT NULL THEN {present} ELSE {absent} END"
                ))
            }
            ScalarFunction::Coalesce | ScalarFunction::IfNull => {
                let branches = arguments
                    .iter()
                    .map(|argument| self.cast_branch(argument, result))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(format!("coalesce({})", branches.join(", ")))
            }
            ScalarFunction::Nullif => self.call("nullif", arguments),
            ScalarFunction::ZeroIfNull => {
                let [value] = self.exactly(arguments)?;
                Ok(format!("coalesce({}, 0)", self.write(value)?))
            }
            ScalarFunction::NullIfZero => {
                let [value] = self.exactly(arguments)?;
                Ok(format!("nullif({}, 0)", self.write(value)?))
            }
            _ => Err(self.error(InvariantViolation::MalformedFunctionCall)),
        }
    }

    /// `CAST(branch AS result)`.
    fn cast_branch(&mut self, branch: &Expression, result: &str) -> Result<String, GenerateError> {
        Ok(format!("CAST({} AS {result})", self.write(branch)?))
    }
}

//! Comparisons, with the `date` bounds of a `timestamp` converted to the start of their day
//! (E6-4.2b).

use menura_contracts::types::common::LogicalType;
use menura_model::expression::{BinaryOperator, Expression, Literal};

use super::operators::operator_sql;
use super::{ExpressionWriter, Literals};
use crate::error::GenerateError;

impl ExpressionWriter<'_, '_> {
    /// `l op r`, each side converted when it is a `date` parameter and the other a timestamp.
    pub(super) fn comparison(
        &mut self,
        operator: BinaryOperator,
        left: &Expression,
        right: &Expression,
    ) -> Result<String, GenerateError> {
        let left_type = self.known_type(left);
        let right_type = self.known_type(right);
        let left = self.comparand(left, right_type.as_ref())?;
        let right = self.comparand(right, left_type.as_ref())?;
        Ok(format!("{left} {} {right}", operator_sql(operator)))
    }

    /// `expression` as an operand compared with a value of `other` type: a `date` parameter
    /// compared with a timestamp becomes the start of that day,
    /// `CAST(CAST($n AS DATE) AS TIMESTAMP)`, in the query time zone for a timestamp with time
    /// zone (E6-4.2b, E6-4.10).
    pub(super) fn comparand(
        &mut self,
        expression: &Expression,
        other: Option<&LogicalType>,
    ) -> Result<String, GenerateError> {
        let text = self.operand(expression)?;
        let date_parameter = self.literals == Literals::Bound
            && matches!(expression, Expression::Literal(Literal::Date(_)));
        match other {
            Some(LogicalType::Timestamp { with_time_zone, .. }) if date_parameter => {
                let midnight = format!("CAST({text} AS TIMESTAMP)");
                if *with_time_zone {
                    Ok(format!("{midnight} AT TIME ZONE {}", self.time_zone()))
                } else {
                    Ok(midnight)
                }
            }
            _ => Ok(text),
        }
    }
}

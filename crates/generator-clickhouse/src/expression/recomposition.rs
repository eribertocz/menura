//! Recomposed components over a summary (SUM-3.1, E6-4.15). The node casts each result to the
//! result type of the base component (SUM-3.8).

use menura_contracts::types::common::LogicalType;
use menura_model::expression::{AggregateCall, Expression, Identifier, Reference, SimpleAggregate};
use menura_representations::logical_plan::Recomposition;
use menura_representations::names::ColumnName;

use super::ExpressionWriter;
use crate::error::{GenerateError, InvariantViolation};

impl ExpressionWriter<'_, '_> {
    /// The SQL of `call`, over a summary measure, recomposed as `recomposition`: `SUM` and
    /// `MIN` or `MAX` as written, `COUNT` as `COALESCE(SUM(c), 0)`, and `AVG` as the exact
    /// recomposed sum divided by the recomposed count, in 64-bit floats (E0F-5), NULL when the
    /// count is 0 (E6-4.6).
    ///
    /// # Errors
    ///
    /// [`InvariantViolation::MalformedRecomposition`] when `call` is not the aggregate the
    /// recomposition reads, or `filter` is set.
    pub(crate) fn recomposed(
        &mut self,
        call: &AggregateCall,
        filter: Option<&Expression>,
        recomposition: &Recomposition,
    ) -> Result<String, GenerateError> {
        let (function, argument) = match call {
            AggregateCall::Simple { function, argument } if filter.is_none() => {
                (*function, argument)
            }
            _ => return Err(self.error(InvariantViolation::MalformedRecomposition)),
        };
        let reads = match recomposition {
            Recomposition::MinMax => {
                matches!(
                    function,
                    SimpleAggregate::Minimum | SimpleAggregate::Maximum
                )
            }
            Recomposition::Sum | Recomposition::Count | Recomposition::Average { .. } => {
                function == SimpleAggregate::Sum
            }
        };
        if !reads {
            return Err(self.error(InvariantViolation::MalformedRecomposition));
        }
        let integer = matches!(self.known_type(argument), Some(LogicalType::Integer { .. }));
        let measure = self.write(argument)?;
        match recomposition {
            Recomposition::Sum => Ok(format!("sumOrNull({})", widened(&measure, integer))),
            Recomposition::Count => Ok(count_sql(&measure)),
            Recomposition::MinMax => {
                let name = if function == SimpleAggregate::Minimum {
                    "minOrNull"
                } else {
                    "maxOrNull"
                };
                Ok(format!("{name}({measure})"))
            }
            Recomposition::Average { count } => {
                let count = count_sql(&self.column(count)?);
                Ok(format!(
                    "CASE WHEN {count} = 0 THEN NULL \
                     ELSE toFloat64(sum({measure})) / toFloat64({count}) END"
                ))
            }
        }
    }

    /// The SQL of the input column `name`.
    fn column(&mut self, name: &ColumnName) -> Result<String, GenerateError> {
        let identifier = Identifier::delimited(name.as_str())
            .ok_or_else(|| self.error(InvariantViolation::UnknownColumn))?;
        self.write(&Expression::Reference(Reference {
            qualifier: None,
            name: identifier,
        }))
    }
}

/// The recomposed count of the summary measure `measure`: `0` over no rows (SUM-3.1).
fn count_sql(measure: &str) -> String {
    format!("coalesce(sum(toInt128({measure})), 0)")
}

/// `measure` widened to `Int128` when it is an integer, so that its sum cannot wrap around.
fn widened(measure: &str, integer: bool) -> String {
    if integer {
        format!("toInt128({measure})")
    } else {
        measure.to_owned()
    }
}

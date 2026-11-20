//! Aggregate calls (E0F-5, E0F-10.5). The node casts each result to its fixed type (E6-4.5).

use menura_model::expression::{
    AggregateCall, DistinctAggregate, Expression, Literal, PercentileFunction, SimpleAggregate,
    SortDirection,
};

use super::ExpressionWriter;
use crate::error::{GenerateError, InvariantViolation};
use crate::values::literal_sql;

impl ExpressionWriter<'_, '_> {
    /// The SQL of `call` with its optional `FILTER (WHERE …)`.
    pub(crate) fn aggregate(
        &mut self,
        call: &AggregateCall,
        filter: Option<&Expression>,
    ) -> Result<String, GenerateError> {
        match call {
            AggregateCall::CountRows => self.filtered("COUNT(*)".to_owned(), filter),
            AggregateCall::Simple { function, argument } => {
                self.simple(*function, argument, filter)
            }
            AggregateCall::Distinct { function, argument } => {
                let name = match function {
                    DistinctAggregate::Sum => "SUM",
                    DistinctAggregate::Count => "COUNT",
                };
                let argument = self.write(argument)?;
                self.filtered(format!("{name}(DISTINCT {argument})"), filter)
            }
            AggregateCall::Percentile {
                function,
                fraction,
                ordering,
                direction,
            } => self.percentile(*function, fraction, ordering, *direction, filter),
        }
    }

    fn simple(
        &mut self,
        function: SimpleAggregate,
        argument: &Expression,
        filter: Option<&Expression>,
    ) -> Result<String, GenerateError> {
        if function == SimpleAggregate::Average {
            return self.average(argument, filter);
        }
        let argument = self.write(argument)?;
        let call = match function {
            SimpleAggregate::Median => format!("MEDIAN(CAST({argument} AS DOUBLE))"),
            _ => format!("{}({argument})", simple_name(function)),
        };
        self.filtered(call, filter)
    }

    /// `AVG` as the exact sum divided by the count, in 64-bit floats (E0F-5); NULL over no
    /// rows, as `AVG`.
    fn average(
        &mut self,
        argument: &Expression,
        filter: Option<&Expression>,
    ) -> Result<String, GenerateError> {
        let argument = self.write(argument)?;
        let filter = self.filter_clause(filter)?;
        let count = format!("COUNT({argument}){filter}");
        let sum = format!("SUM({argument}){filter}");
        Ok(format!(
            "CASE WHEN {count} = 0 THEN NULL ELSE CAST({sum} AS DOUBLE) / CAST({count} AS DOUBLE) END"
        ))
    }

    /// `PERCENTILE_CONT(p) WITHIN GROUP (ORDER BY x)`, `x` cast to a 64-bit float first. The
    /// fraction is a numeric literal already validated (E0F-10.5, E6-4.11).
    fn percentile(
        &mut self,
        function: PercentileFunction,
        fraction: &Expression,
        ordering: &Expression,
        direction: SortDirection,
        filter: Option<&Expression>,
    ) -> Result<String, GenerateError> {
        let fraction = match fraction {
            Expression::Literal(literal @ (Literal::Integer(_) | Literal::Decimal(_))) => {
                validated_number(literal)
            }
            _ => return Err(self.error(InvariantViolation::ExpressionOutsideSlice)),
        };
        let name = match function {
            PercentileFunction::Continuous => "PERCENTILE_CONT",
            PercentileFunction::Discrete => "PERCENTILE_DISC",
        };
        let ordering = self.write(ordering)?;
        let direction = match direction {
            SortDirection::Ascending => "ASC",
            SortDirection::Descending => "DESC",
        };
        let call = format!(
            "{name}({fraction}) WITHIN GROUP (ORDER BY CAST({ordering} AS DOUBLE) {direction})"
        );
        self.filtered(call, filter)
    }

    fn filtered(
        &mut self,
        call: String,
        filter: Option<&Expression>,
    ) -> Result<String, GenerateError> {
        Ok(call + &self.filter_clause(filter)?)
    }

    fn filter_clause(&mut self, filter: Option<&Expression>) -> Result<String, GenerateError> {
        match filter {
            Some(condition) => Ok(format!(" FILTER (WHERE {})", self.write(condition)?)),
            None => Ok(String::new()),
        }
    }
}

/// The plain digits of a validated integer or decimal literal.
fn validated_number(literal: &Literal) -> String {
    match literal {
        Literal::Integer(number) => number.digits().to_owned(),
        Literal::Decimal(number) => {
            let integer = if number.integer_digits().is_empty() {
                "0"
            } else {
                number.integer_digits()
            };
            if number.fraction_digits().is_empty() {
                integer.to_owned()
            } else {
                format!("{integer}.{}", number.fraction_digits())
            }
        }
        _ => literal_sql(literal),
    }
}

/// The `DuckDB` aggregate of `function`. `STDDEV` and `VARIANCE` are the sample statistics.
fn simple_name(function: SimpleAggregate) -> &'static str {
    match function {
        SimpleAggregate::Sum => "SUM",
        SimpleAggregate::Count => "COUNT",
        SimpleAggregate::Average => "AVG",
        SimpleAggregate::Minimum => "MIN",
        SimpleAggregate::Maximum => "MAX",
        SimpleAggregate::StandardDeviation | SimpleAggregate::SampleStandardDeviation => {
            "STDDEV_SAMP"
        }
        SimpleAggregate::PopulationStandardDeviation => "STDDEV_POP",
        SimpleAggregate::Variance | SimpleAggregate::SampleVariance => "VAR_SAMP",
        SimpleAggregate::PopulationVariance => "VAR_POP",
        SimpleAggregate::Median => "MEDIAN",
    }
}

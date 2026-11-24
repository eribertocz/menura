//! Aggregate calls (E0F-5, E0F-10.5). The node casts each result to its fixed type (E6-4.5).
//!
//! `ClickHouse` has no `FILTER (WHERE …)`: a filtered aggregate reads
//! `if(ifNull(condition, 0), x, NULL)`, which every aggregate skips where the condition does
//! not hold. Over no value, an aggregate other than `COUNT` is NULL as ISO 9075 states, where
//! `ClickHouse` returns a default value or `nan`: `aggregate_functions_null_for_empty = 1`
//! (E6-7.8) makes it NULL, and the `-OrNull` combinators and count guards say so explicitly as
//! well. A count over no value is 0, read through `coalesce` whatever that setting does to
//! `count`.

use menura_contracts::types::common::LogicalType;
use menura_model::expression::{
    AggregateCall, DistinctAggregate, Expression, Literal, PercentileFunction, SimpleAggregate,
    SortDirection,
};

use super::ExpressionWriter;
use crate::error::{GenerateError, InvariantViolation};

impl ExpressionWriter<'_, '_> {
    /// The SQL of `call` restricted to the rows where `filter` holds.
    pub(crate) fn aggregate(
        &mut self,
        call: &AggregateCall,
        filter: Option<&Expression>,
    ) -> Result<String, GenerateError> {
        match call {
            AggregateCall::CountRows => match filter {
                Some(condition) => Ok(format!(
                    "coalesce(countIf(ifNull({}, 0)), 0)",
                    self.write(condition)?
                )),
                None => Ok("coalesce(count(), 0)".to_owned()),
            },
            AggregateCall::Simple { function, argument } => {
                self.simple(*function, argument, filter)
            }
            AggregateCall::Distinct { function, argument } => {
                let value = self.summed(argument, filter)?;
                Ok(match function {
                    DistinctAggregate::Sum => format!(
                        "CASE WHEN coalesce(count({value}), 0) = 0 THEN NULL ELSE sum(DISTINCT {value}) END"
                    ),
                    DistinctAggregate::Count => format!("coalesce(count(DISTINCT {value}), 0)"),
                })
            }
            AggregateCall::Percentile {
                function,
                fraction,
                ordering,
                direction,
            } => self.percentile(*function, fraction, ordering, *direction, filter),
        }
    }

    /// An aggregate without `DISTINCT`.
    fn simple(
        &mut self,
        function: SimpleAggregate,
        argument: &Expression,
        filter: Option<&Expression>,
    ) -> Result<String, GenerateError> {
        Ok(match function {
            SimpleAggregate::Count => {
                format!("coalesce(count({}), 0)", self.argument(argument, filter)?)
            }
            SimpleAggregate::Sum => format!("sumOrNull({})", self.summed(argument, filter)?),
            SimpleAggregate::Minimum => format!("minOrNull({})", self.argument(argument, filter)?),
            SimpleAggregate::Maximum => format!("maxOrNull({})", self.argument(argument, filter)?),
            SimpleAggregate::Average => {
                let value = self.argument(argument, filter)?;
                let summed = self.widened_sum(argument, &value);
                format!(
                    "CASE WHEN coalesce(count({value}), 0) = 0 THEN NULL \
                     ELSE toFloat64(sum({summed})) / toFloat64(count({value})) END"
                )
            }
            SimpleAggregate::Median => {
                let value = self.float_argument(argument, filter)?;
                at_least(&value, 1, &format!("quantileExactInclusive(0.5)({value})"))
            }
            SimpleAggregate::StandardDeviation | SimpleAggregate::SampleStandardDeviation => {
                self.statistic("stddevSamp", 2, argument, filter)?
            }
            SimpleAggregate::PopulationStandardDeviation => {
                self.statistic("stddevPop", 1, argument, filter)?
            }
            SimpleAggregate::Variance | SimpleAggregate::SampleVariance => {
                self.statistic("varSamp", 2, argument, filter)?
            }
            SimpleAggregate::PopulationVariance => self.statistic("varPop", 1, argument, filter)?,
        })
    }

    /// A statistic of the 64-bit float values, NULL over fewer than `minimum` values, where
    /// `ClickHouse` returns `nan` or an infinity.
    fn statistic(
        &mut self,
        name: &str,
        minimum: u8,
        argument: &Expression,
        filter: Option<&Expression>,
    ) -> Result<String, GenerateError> {
        let value = self.float_argument(argument, filter)?;
        Ok(at_least(&value, minimum, &format!("{name}({value})")))
    }

    /// `PERCENTILE_CONT(p)` as `quantileExactInclusive(p)`, which interpolates between the
    /// two closest ranks as `PERCENTILE_CONT` does, and `PERCENTILE_DISC(p)` as the value at
    /// rank `ceil(n × p)` (at least 1) of the sorted values, computed in integers. `x` is cast
    /// to a 64-bit float first. A descending order reads `1 − p` in ascending order for the
    /// continuous percentile, and the values sorted descending for the discrete one. The
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
            Expression::Literal(Literal::Integer(number)) => Fraction::new(number.digits(), ""),
            Expression::Literal(Literal::Decimal(number)) => {
                Fraction::new(number.integer_digits(), number.fraction_digits())
            }
            _ => None,
        }
        .ok_or_else(|| self.error(InvariantViolation::ExpressionOutsideSlice))?;
        let value = self.float_argument(ordering, filter)?;
        let call = match (function, direction) {
            (PercentileFunction::Continuous, SortDirection::Ascending) => {
                format!("quantileExactInclusive({})({value})", fraction.text())
            }
            (PercentileFunction::Continuous, SortDirection::Descending) => format!(
                "quantileExactInclusive({})({value})",
                fraction.complement().text()
            ),
            (PercentileFunction::Discrete, direction) => {
                let sort = match direction {
                    SortDirection::Ascending => "arraySort",
                    SortDirection::Descending => "arrayReverseSort",
                };
                format!(
                    "{sort}(groupArray({value}))[greatest(intDiv(count({value}) * {} + {}, {}), 1)]",
                    fraction.numerator,
                    fraction.denominator - 1,
                    fraction.denominator
                )
            }
        };
        Ok(at_least(&value, 1, &call))
    }

    /// The argument of an aggregate, NULL where `filter` does not hold.
    fn argument(
        &mut self,
        argument: &Expression,
        filter: Option<&Expression>,
    ) -> Result<String, GenerateError> {
        let value = self.write(argument)?;
        match filter {
            Some(condition) => Ok(format!(
                "if(ifNull({}, 0), {value}, NULL)",
                self.write(condition)?
            )),
            None => Ok(value),
        }
    }

    /// The argument of `SUM`: an integer widened to `Int256` first and checked into the
    /// `Int128` of E0F-5, so that the sum cannot wrap around (E6-7.9).
    fn summed(
        &mut self,
        argument: &Expression,
        filter: Option<&Expression>,
    ) -> Result<String, GenerateError> {
        let value = self.argument(argument, filter)?;
        Ok(self.widened_sum(argument, &value))
    }

    /// `value`, the written `argument`, widened to `Int256` when `argument` is an integer.
    fn widened_sum(&self, argument: &Expression, value: &str) -> String {
        if matches!(self.known_type(argument), Some(LogicalType::Integer { .. })) {
            format!("toInt256({value})")
        } else {
            value.to_owned()
        }
    }

    /// The argument of an aggregate as a 64-bit float.
    fn float_argument(
        &mut self,
        argument: &Expression,
        filter: Option<&Expression>,
    ) -> Result<String, GenerateError> {
        Ok(format!("toFloat64({})", self.argument(argument, filter)?))
    }
}

/// `call`, NULL when `value` has fewer than `minimum` values that are not NULL.
fn at_least(value: &str, minimum: u8, call: &str) -> String {
    format!("CASE WHEN coalesce(count({value}), 0) < {minimum} THEN NULL ELSE {call} END")
}

/// A validated percentile fraction `numerator / denominator`, the denominator a power of ten.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Fraction {
    numerator: u128,
    denominator: u128,
}

impl Fraction {
    /// The fraction of the literal with these integer and fraction digits, or `None` when it
    /// has too many digits to be computed exactly.
    fn new(integer_digits: &str, fraction_digits: &str) -> Option<Self> {
        let scale = u32::try_from(fraction_digits.len()).ok()?;
        let denominator = 10_u128.checked_pow(scale)?;
        let digits = format!("{integer_digits}{fraction_digits}");
        let numerator = if digits.trim_start_matches('0').is_empty() {
            0
        } else {
            digits.trim_start_matches('0').parse().ok()?
        };
        Some(Self {
            numerator,
            denominator,
        })
    }

    /// `1 − p`, exact.
    fn complement(self) -> Self {
        Self {
            numerator: self.denominator.saturating_sub(self.numerator),
            denominator: self.denominator,
        }
    }

    /// The fraction as a decimal literal, such as `0.25`.
    fn text(self) -> String {
        let scale = self.denominator.to_string().len() - 1;
        if scale == 0 {
            return self.numerator.to_string();
        }
        let digits = format!("{:0>width$}", self.numerator, width = scale + 1);
        let (integer, fraction) = digits.split_at(digits.len() - scale);
        format!("{integer}.{fraction}")
    }
}

#[cfg(test)]
mod tests {
    use super::Fraction;

    #[test]
    fn e6_7_9_a_fraction_and_its_complement_are_exact() {
        let fraction = Fraction::new("0", "25").unwrap();
        assert_eq!(fraction.text(), "0.25");
        assert_eq!(fraction.complement().text(), "0.75");
        assert_eq!(Fraction::new("1", "").unwrap().complement().text(), "0");
        assert_eq!(Fraction::new("", "05").unwrap().text(), "0.05");
    }
}

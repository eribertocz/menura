//! Types aggregates (E0F-5, E0F-10.5) and checks where they appear (E0F-07).

use menura_contracts::types::common::LogicalType;
use menura_model::expression::{
    AggregateCall, DistinctAggregate, Expression, Literal, SimpleAggregate,
};

use super::code::TypingCode;
use super::stop::{Outcome, incompatible, problem, untyped_null};
use super::typer::{Aggregates, TypingScope, expect_numeric};
use super::value_type::{Numeric, Typed, big_integer, decimal, double, huge_integer};

impl TypingScope<'_> {
    /// The type of an aggregate call with its optional `FILTER (WHERE …)`.
    pub(super) fn aggregate(
        &self,
        call: &AggregateCall,
        filter: Option<&Expression>,
    ) -> Outcome<Typed> {
        match self.aggregates {
            Aggregates::Allowed => {}
            Aggregates::NotAllowed => {
                return problem(
                    TypingCode::MisplacedAggregate,
                    "an aggregate in a field expression".to_owned(),
                );
            }
            Aggregates::Nested => {
                return problem(
                    TypingCode::MisplacedAggregate,
                    "an aggregate inside another aggregate".to_owned(),
                );
            }
        }
        let inner = self.nested();
        let value_type = match call {
            AggregateCall::CountRows => big_integer(),
            AggregateCall::Simple { function, argument } => {
                simple(*function, &inner.type_of(argument)?)?
            }
            AggregateCall::Distinct { function, argument } => {
                let argument = inner.type_of(argument)?;
                match function {
                    DistinctAggregate::Count => big_integer(),
                    DistinctAggregate::Sum => sum(&argument)?,
                }
            }
            AggregateCall::Percentile {
                fraction, ordering, ..
            } => percentile(
                &inner.type_of(fraction)?,
                fraction,
                &inner.type_of(ordering)?,
            )?,
        };
        if let Some(filter) = filter {
            inner.condition(filter, "the FILTER condition")?;
        }
        Ok(Typed::value(value_type, can_be_null(call)))
    }
}

/// Whether the aggregate can be NULL: every aggregate but `COUNT` gives NULL over no row
/// (E0F-7.1).
fn can_be_null(call: &AggregateCall) -> bool {
    !matches!(
        call,
        AggregateCall::CountRows
            | AggregateCall::Simple {
                function: SimpleAggregate::Count,
                ..
            }
            | AggregateCall::Distinct {
                function: DistinctAggregate::Count,
                ..
            }
    )
}

/// The result type of an aggregate without `DISTINCT` (E0F-5).
fn simple(function: SimpleAggregate, argument: &Typed) -> Outcome<LogicalType> {
    match function {
        SimpleAggregate::Count => Ok(big_integer()),
        SimpleAggregate::Sum => sum(argument),
        SimpleAggregate::Minimum | SimpleAggregate::Maximum => match argument.logical() {
            Some(value_type) => Ok(value_type.clone()),
            None => untyped_null("the argument of MIN and MAX"),
        },
        SimpleAggregate::Median => median_argument(argument),
        _ => {
            expect_numeric(argument, "the argument of AVG, STDDEV and VARIANCE")?;
            Ok(double())
        }
    }
}

/// `SUM`: integer to 128-bit integer, `decimal(p,s)` to `decimal(38,s)`, float to float.
fn sum(argument: &Typed) -> Outcome<LogicalType> {
    match expect_numeric(argument, "the argument of SUM")? {
        Some(Numeric::Integer { .. }) => Ok(huge_integer()),
        Some(Numeric::Decimal { scale, .. }) => Ok(decimal(38, scale)),
        Some(Numeric::Float) => Ok(double()),
        None => untyped_null("the argument of SUM"),
    }
}

/// `MEDIAN` and the percentiles: an integer, decimal or float, giving a 64-bit float (E0F-5).
fn median_argument(argument: &Typed) -> Outcome<LogicalType> {
    expect_numeric(argument, "the argument of MEDIAN and the percentiles")?;
    Ok(double())
}

/// `PERCENTILE_CONT(p)` and `PERCENTILE_DISC(p)`: `p` a numeric literal `0 ≤ p ≤ 1`
/// (E0F-10.5); any other `p` is `E0F-03` (E0F-5a).
fn percentile(fraction: &Typed, written: &Expression, ordering: &Typed) -> Outcome<LogicalType> {
    expect_numeric(fraction, "the fraction of a percentile")?;
    if !is_fraction_literal(written) {
        return incompatible(
            "the fraction of a percentile must be a number literal from 0 to 1".to_owned(),
        );
    }
    median_argument(ordering)
}

/// Whether `expression` is a number literal from 0 to 1.
fn is_fraction_literal(expression: &Expression) -> bool {
    match expression {
        Expression::Literal(Literal::Integer(number)) => at_most_one(number.digits(), 0),
        Expression::Literal(Literal::Decimal(number)) => at_most_one(
            &format!("{}{}", number.integer_digits(), number.fraction_digits()),
            -exponent_of(number.fraction_digits()),
        ),
        Expression::Literal(Literal::Float(number)) => at_most_one(
            &format!("{}{}", number.integer_digits(), number.fraction_digits()),
            i64::from(number.exponent()) - exponent_of(number.fraction_digits()),
        ),
        _ => false,
    }
}

/// The number of `digits`, as a power of ten.
fn exponent_of(digits: &str) -> i64 {
    i64::try_from(digits.len()).unwrap_or(i64::MAX)
}

/// Whether the non-negative number `digits × 10^exponent` is at most 1, computed exactly.
fn at_most_one(digits: &str, exponent: i64) -> bool {
    let significant = digits.trim_start_matches('0');
    let trimmed = significant.trim_end_matches('0');
    if trimmed.is_empty() {
        return true;
    }
    let dropped = exponent_of(significant) - exponent_of(trimmed);
    let integer_digits = exponent_of(trimmed).saturating_add(exponent.saturating_add(dropped));
    integer_digits <= 0 || (integer_digits == 1 && trimmed == "1")
}

//! Types aggregates (E0F-5, E0F-10.5) and checks where they appear (E0F-07).

use semantic_compiler_contracts::types::common::LogicalType;
use semantic_compiler_model::expression::{
    AggregateCall, DistinctAggregate, Expression, Literal, SimpleAggregate,
};

use super::diagnostic::TypingCode;
use super::stop::{Outcome, Stop, problem};
use super::typer::{Aggregates, ExpressionTyper, expect_numeric};
use super::value_type::{Numeric, Typed, big_integer, decimal, double, huge_integer};

impl ExpressionTyper<'_> {
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
        SimpleAggregate::Minimum | SimpleAggregate::Maximum => {
            argument.logical().cloned().ok_or(Stop::Undecided)
        }
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
        None => Err(Stop::Undecided),
    }
}

/// `MEDIAN` and the percentiles: an integer or decimal, cast to a 64-bit float. A float
/// argument is issue #126.
fn median_argument(argument: &Typed) -> Outcome<LogicalType> {
    match expect_numeric(argument, "the argument of MEDIAN and the percentiles")? {
        Some(Numeric::Float) => Err(Stop::Undecided),
        _ => Ok(double()),
    }
}

/// `PERCENTILE_CONT(p)` and `PERCENTILE_DISC(p)`: `p` a numeric literal `0 ≤ p ≤ 1`
/// (E0F-10.5); any other `p` is issue #126.
fn percentile(fraction: &Typed, written: &Expression, ordering: &Typed) -> Outcome<LogicalType> {
    expect_numeric(fraction, "the fraction of a percentile")?;
    if !is_fraction_literal(written) {
        return Err(Stop::Undecided);
    }
    median_argument(ordering)
}

/// Whether `expression` is a number literal from 0 to 1.
fn is_fraction_literal(expression: &Expression) -> bool {
    let (integer_digits, fraction_digits) = match expression {
        Expression::Literal(Literal::Integer(number)) => (number.digits(), ""),
        Expression::Literal(Literal::Decimal(number)) => {
            (number.integer_digits(), number.fraction_digits())
        }
        _ => return false,
    };
    let integer = integer_digits.trim_start_matches('0');
    integer.is_empty() || (integer == "1" && fraction_digits.bytes().all(|digit| digit == b'0'))
}

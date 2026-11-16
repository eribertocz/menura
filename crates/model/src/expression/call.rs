//! Function and aggregate calls of the accepted subset (E0F-2.2).

use super::tree::Expression;

/// A scalar function of the accepted subset.
///
/// The list is provisional until issue #51 settles the `OSSIE_SQL_2026` REQUIRED functions: it
/// holds only the functions the specification names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScalarFunction {
    /// `COALESCE(a, b, …)`, two or more arguments.
    Coalesce,
    /// `NULLIF(a, b)`.
    Nullif,
    /// `ROUND(x, n)`, rounding half away from zero (E0F-2.5).
    Round,
}

/// An aggregate call.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AggregateCall {
    /// `COUNT(*)`.
    CountRows,
    /// `FUNCTION(argument)`.
    Simple {
        /// The aggregate.
        function: SimpleAggregate,
        /// The aggregated value.
        argument: Box<Expression>,
    },
    /// `SUM(DISTINCT argument)` or `COUNT(DISTINCT argument)`.
    Distinct {
        /// The aggregate.
        function: DistinctAggregate,
        /// The aggregated value.
        argument: Box<Expression>,
    },
    /// `PERCENTILE_CONT(fraction) WITHIN GROUP (ORDER BY ordering [ASC | DESC])`, or the
    /// `PERCENTILE_DISC` form.
    Percentile {
        /// The aggregate.
        function: PercentileFunction,
        /// The requested fraction.
        fraction: Box<Expression>,
        /// The ordered value.
        ordering: Box<Expression>,
        /// The order; no direction means ascending.
        direction: SortDirection,
    },
}

/// An aggregate that takes one value and no `DISTINCT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimpleAggregate {
    /// `SUM`.
    Sum,
    /// `COUNT`.
    Count,
    /// `AVG`.
    Average,
    /// `MIN`.
    Minimum,
    /// `MAX`.
    Maximum,
    /// `STDDEV`.
    StandardDeviation,
    /// `STDDEV_POP`.
    PopulationStandardDeviation,
    /// `STDDEV_SAMP`.
    SampleStandardDeviation,
    /// `VARIANCE`.
    Variance,
    /// `VAR_POP`.
    PopulationVariance,
    /// `VAR_SAMP`.
    SampleVariance,
    /// `MEDIAN`.
    Median,
}

/// An aggregate that accepts `DISTINCT` (E0F-2.2: `SUM(DISTINCT)` and `COUNT(DISTINCT)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DistinctAggregate {
    /// `SUM(DISTINCT …)`.
    Sum,
    /// `COUNT(DISTINCT …)`.
    Count,
}

/// An ordered-set percentile aggregate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PercentileFunction {
    /// `PERCENTILE_CONT`, interpolated.
    Continuous,
    /// `PERCENTILE_DISC`, a value of the input.
    Discrete,
}

/// A sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortDirection {
    /// `ASC`, the default.
    Ascending,
    /// `DESC`.
    Descending,
}

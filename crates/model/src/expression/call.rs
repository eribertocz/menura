//! Function and aggregate calls of the accepted subset (E0F-2.2, E0F-10).

use super::tree::Expression;

/// A scalar function of E0F-10 called with a plain argument list.
///
/// Name synonyms share one variant (`CEIL` and `CEILING`, `TRUNC` and `TRUNCATE`, `IF` and
/// `IFF`, `IFNULL` and `NVL`), and `CHARINDEX(a, s)` is stored as `POSITION(a IN s)`. The
/// functions with a date part (`EXTRACT`, `DATE_PART`, `DATE_TRUNC`, `DATEADD`, `DATEDIFF`) have
/// their own nodes, and `CURRENT_*` are nodes without arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScalarFunction {
    /// `YEAR(x)`.
    Year,
    /// `QUARTER(x)`.
    Quarter,
    /// `MONTH(x)`.
    Month,
    /// `DAY(x)`.
    Day,
    /// `DAYOFYEAR(x)`.
    DayOfYear,
    /// `HOUR(x)`.
    Hour,
    /// `MINUTE(x)`.
    Minute,
    /// `SECOND(x)`.
    Second,
    /// `TO_DATE(s)`.
    ToDate,
    /// `TO_TIMESTAMP(s)`.
    ToTimestamp,
    /// `CONCAT(a, …)`.
    Concat,
    /// `LENGTH(s)`.
    Length,
    /// `LOWER(s)`.
    Lower,
    /// `UPPER(s)`.
    Upper,
    /// `TRIM(s)`.
    Trim,
    /// `LTRIM(s)`.
    LeftTrim,
    /// `RTRIM(s)`.
    RightTrim,
    /// `LEFT(s, n)`.
    Left,
    /// `RIGHT(s, n)`.
    Right,
    /// `SUBSTRING(s, start[, length])`.
    Substring,
    /// `REPLACE(s, from, to)`.
    Replace,
    /// `SPLIT_PART(s, delimiter, n)`.
    SplitPart,
    /// `POSITION(a IN s)` or `CHARINDEX(a, s)`; the arguments are `[a, s]`.
    Position,
    /// `CONTAINS(s, a)`.
    Contains,
    /// `STARTSWITH(s, a)`.
    StartsWith,
    /// `ENDSWITH(s, a)`.
    EndsWith,
    /// `ABS(x)`.
    Absolute,
    /// `SIGN(x)`.
    Sign,
    /// `FLOOR(x)`.
    Floor,
    /// `CEIL(x)` or `CEILING(x)`.
    Ceiling,
    /// `ROUND(x[, n])`, rounding half away from zero (E0F-2.5).
    Round,
    /// `TRUNC(x[, n])` or `TRUNCATE(x[, n])`.
    Truncate,
    /// `MOD(x, y)`.
    Modulo,
    /// `POWER(x, y)`.
    Power,
    /// `SQRT(x)`.
    SquareRoot,
    /// `EXP(x)`.
    Exponential,
    /// `LN(x)`.
    NaturalLogarithm,
    /// `LOG(base, x)`.
    Logarithm,
    /// `LOG10(x)`.
    DecimalLogarithm,
    /// `GREATEST(a, b, …)`.
    Greatest,
    /// `LEAST(a, b, …)`.
    Least,
    /// `IF(c, a, b)` or `IFF(c, a, b)`.
    If,
    /// `NULLIF(a, b)`.
    Nullif,
    /// `COALESCE(a, b, …)`.
    Coalesce,
    /// `IFNULL(a, b)` or `NVL(a, b)`.
    IfNull,
    /// `NVL2(a, b, c)`.
    Nvl2,
    /// `ZEROIFNULL(x)`.
    ZeroIfNull,
    /// `NULLIFZERO(x)`.
    NullIfZero,
}

/// The part of `EXTRACT(part FROM x)` and `DATE_PART('part', x)` (E0F-10.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtractPart {
    /// `YEAR`.
    Year,
    /// `QUARTER`.
    Quarter,
    /// `MONTH`.
    Month,
    /// `WEEK`: the ISO 8601 week, 1 to 53.
    Week,
    /// `DAY`.
    Day,
    /// `DAYOFWEEK`: ISO, Monday 1 to Sunday 7.
    DayOfWeek,
    /// `DAYOFYEAR`.
    DayOfYear,
    /// `HOUR`.
    Hour,
    /// `MINUTE`.
    Minute,
    /// `SECOND`, without fraction.
    Second,
}

/// The unit of `DATE_TRUNC`, `DATEADD` and `DATEDIFF` (E0F-10.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DateUnit {
    /// `year`.
    Year,
    /// `quarter`.
    Quarter,
    /// `month`.
    Month,
    /// `week`, starting on Monday.
    Week,
    /// `day`.
    Day,
    /// `hour`.
    Hour,
    /// `minute`.
    Minute,
    /// `second`.
    Second,
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
        /// The order; no direction means ascending. `NULLS FIRST` and `NULLS LAST` have no
        /// effect (E0F-10.5) and are not kept.
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

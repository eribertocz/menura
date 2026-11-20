//! The closed vocabulary of the accepted subset: reserved words (E0F-9.3), scalar functions
//! and their arities (E0F-10), aggregates (E0F-10.5), date parts (E0F-10.1) and `CAST` target
//! types (E0F-11). Each list is one table here.

use menura_model::expression::{
    CastTarget, DateUnit, ExtractPart, ScalarFunction, SimpleAggregate,
};

/// Words that are never a regular identifier: a reference must write them as delimited
/// identifiers (`"end"`). Other keywords, such as `DATE`, `FILTER` or `ORDER`, are recognized
/// only where the grammar expects them.
const RESERVED_WORDS: [&str; 20] = [
    "AND", "AS", "BETWEEN", "CASE", "CAST", "DISTINCT", "ELSE", "END", "FALSE", "FROM", "ILIKE",
    "IN", "IS", "LIKE", "NOT", "NULL", "OR", "THEN", "TRUE", "WHEN",
];

/// Whether `word` is `keyword`, ignoring ASCII case.
pub(crate) fn is_keyword(word: &str, keyword: &str) -> bool {
    word.eq_ignore_ascii_case(keyword)
}

/// Whether `word` is reserved and so cannot be a regular identifier.
pub(crate) fn is_reserved(word: &str) -> bool {
    RESERVED_WORDS
        .iter()
        .any(|reserved| is_keyword(word, reserved))
}

/// How many arguments a function takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Arity {
    /// Exactly this many.
    Exactly(usize),
    /// From the first to the second, inclusive.
    Between(usize, usize),
    /// This many or more.
    AtLeast(usize),
}

impl Arity {
    /// Whether `count` arguments are accepted.
    pub(crate) fn accepts(self, count: usize) -> bool {
        match self {
            Self::Exactly(expected) => count == expected,
            Self::Between(minimum, maximum) => (minimum..=maximum).contains(&count),
            Self::AtLeast(minimum) => count >= minimum,
        }
    }
}

/// The scalar functions of E0F-10 with a plain argument list: name, function, arity. The first
/// name of a function is the one [`scalar_function_name`] gives. `POSITION(a IN s)` has its
/// own form and is reached here only as `CHARINDEX(a, s)`.
const SCALAR_FUNCTIONS: [(&str, ScalarFunction, Arity); 52] = [
    ("YEAR", ScalarFunction::Year, Arity::Exactly(1)),
    ("QUARTER", ScalarFunction::Quarter, Arity::Exactly(1)),
    ("MONTH", ScalarFunction::Month, Arity::Exactly(1)),
    ("DAY", ScalarFunction::Day, Arity::Exactly(1)),
    ("DAYOFYEAR", ScalarFunction::DayOfYear, Arity::Exactly(1)),
    ("HOUR", ScalarFunction::Hour, Arity::Exactly(1)),
    ("MINUTE", ScalarFunction::Minute, Arity::Exactly(1)),
    ("SECOND", ScalarFunction::Second, Arity::Exactly(1)),
    ("TO_DATE", ScalarFunction::ToDate, Arity::Exactly(1)),
    (
        "TO_TIMESTAMP",
        ScalarFunction::ToTimestamp,
        Arity::Exactly(1),
    ),
    ("CONCAT", ScalarFunction::Concat, Arity::AtLeast(1)),
    ("LENGTH", ScalarFunction::Length, Arity::Exactly(1)),
    ("LOWER", ScalarFunction::Lower, Arity::Exactly(1)),
    ("UPPER", ScalarFunction::Upper, Arity::Exactly(1)),
    ("TRIM", ScalarFunction::Trim, Arity::Exactly(1)),
    ("LTRIM", ScalarFunction::LeftTrim, Arity::Exactly(1)),
    ("RTRIM", ScalarFunction::RightTrim, Arity::Exactly(1)),
    ("LEFT", ScalarFunction::Left, Arity::Exactly(2)),
    ("RIGHT", ScalarFunction::Right, Arity::Exactly(2)),
    ("SUBSTRING", ScalarFunction::Substring, Arity::Between(2, 3)),
    ("REPLACE", ScalarFunction::Replace, Arity::Exactly(3)),
    ("SPLIT_PART", ScalarFunction::SplitPart, Arity::Exactly(3)),
    ("CHARINDEX", ScalarFunction::Position, Arity::Exactly(2)),
    ("CONTAINS", ScalarFunction::Contains, Arity::Exactly(2)),
    ("STARTSWITH", ScalarFunction::StartsWith, Arity::Exactly(2)),
    ("ENDSWITH", ScalarFunction::EndsWith, Arity::Exactly(2)),
    ("ABS", ScalarFunction::Absolute, Arity::Exactly(1)),
    ("SIGN", ScalarFunction::Sign, Arity::Exactly(1)),
    ("FLOOR", ScalarFunction::Floor, Arity::Exactly(1)),
    ("CEIL", ScalarFunction::Ceiling, Arity::Exactly(1)),
    ("CEILING", ScalarFunction::Ceiling, Arity::Exactly(1)),
    ("ROUND", ScalarFunction::Round, Arity::Between(1, 2)),
    ("TRUNC", ScalarFunction::Truncate, Arity::Between(1, 2)),
    ("TRUNCATE", ScalarFunction::Truncate, Arity::Between(1, 2)),
    ("MOD", ScalarFunction::Modulo, Arity::Exactly(2)),
    ("POWER", ScalarFunction::Power, Arity::Exactly(2)),
    ("SQRT", ScalarFunction::SquareRoot, Arity::Exactly(1)),
    ("EXP", ScalarFunction::Exponential, Arity::Exactly(1)),
    ("LN", ScalarFunction::NaturalLogarithm, Arity::Exactly(1)),
    ("LOG", ScalarFunction::Logarithm, Arity::Exactly(2)),
    ("LOG10", ScalarFunction::DecimalLogarithm, Arity::Exactly(1)),
    ("GREATEST", ScalarFunction::Greatest, Arity::AtLeast(2)),
    ("LEAST", ScalarFunction::Least, Arity::AtLeast(2)),
    ("IF", ScalarFunction::If, Arity::Exactly(3)),
    ("IFF", ScalarFunction::If, Arity::Exactly(3)),
    ("NULLIF", ScalarFunction::Nullif, Arity::Exactly(2)),
    ("COALESCE", ScalarFunction::Coalesce, Arity::AtLeast(2)),
    ("IFNULL", ScalarFunction::IfNull, Arity::Exactly(2)),
    ("NVL", ScalarFunction::IfNull, Arity::Exactly(2)),
    ("NVL2", ScalarFunction::Nvl2, Arity::Exactly(3)),
    ("ZEROIFNULL", ScalarFunction::ZeroIfNull, Arity::Exactly(1)),
    ("NULLIFZERO", ScalarFunction::NullIfZero, Arity::Exactly(1)),
];

/// The scalar function named `word` and its arity, if E0F-10 lists it with a plain argument
/// list.
pub(crate) fn scalar_function(word: &str) -> Option<(ScalarFunction, Arity)> {
    SCALAR_FUNCTIONS
        .into_iter()
        .find_map(|(name, function, arity)| is_keyword(word, name).then_some((function, arity)))
}

/// The name [`crate::print`] writes for a scalar function.
pub(crate) fn scalar_function_name(function: ScalarFunction) -> &'static str {
    SCALAR_FUNCTIONS
        .into_iter()
        .find_map(|(name, candidate, _)| (candidate == function).then_some(name))
        .unwrap_or_default()
}

const SIMPLE_AGGREGATES: [(&str, SimpleAggregate); 12] = [
    ("SUM", SimpleAggregate::Sum),
    ("COUNT", SimpleAggregate::Count),
    ("AVG", SimpleAggregate::Average),
    ("MIN", SimpleAggregate::Minimum),
    ("MAX", SimpleAggregate::Maximum),
    ("STDDEV", SimpleAggregate::StandardDeviation),
    ("STDDEV_POP", SimpleAggregate::PopulationStandardDeviation),
    ("STDDEV_SAMP", SimpleAggregate::SampleStandardDeviation),
    ("VARIANCE", SimpleAggregate::Variance),
    ("VAR_POP", SimpleAggregate::PopulationVariance),
    ("VAR_SAMP", SimpleAggregate::SampleVariance),
    ("MEDIAN", SimpleAggregate::Median),
];

/// The single-argument aggregate named `word`, if E0F-10.5 lists it.
pub(crate) fn simple_aggregate(word: &str) -> Option<SimpleAggregate> {
    SIMPLE_AGGREGATES
        .into_iter()
        .find_map(|(name, aggregate)| is_keyword(word, name).then_some(aggregate))
}

/// The name the specification writes for a single-argument aggregate.
pub(crate) fn simple_aggregate_name(aggregate: SimpleAggregate) -> &'static str {
    SIMPLE_AGGREGATES
        .into_iter()
        .find_map(|(name, candidate)| (candidate == aggregate).then_some(name))
        .unwrap_or_default()
}

const EXTRACT_PARTS: [(&str, ExtractPart); 10] = [
    ("YEAR", ExtractPart::Year),
    ("QUARTER", ExtractPart::Quarter),
    ("MONTH", ExtractPart::Month),
    ("WEEK", ExtractPart::Week),
    ("DAY", ExtractPart::Day),
    ("DAYOFWEEK", ExtractPart::DayOfWeek),
    ("DAYOFYEAR", ExtractPart::DayOfYear),
    ("HOUR", ExtractPart::Hour),
    ("MINUTE", ExtractPart::Minute),
    ("SECOND", ExtractPart::Second),
];

/// The part of `EXTRACT` and `DATE_PART` named `word`, ignoring case.
pub(crate) fn extract_part(word: &str) -> Option<ExtractPart> {
    EXTRACT_PARTS
        .into_iter()
        .find_map(|(name, part)| is_keyword(word, name).then_some(part))
}

/// The name [`crate::print`] writes for an extract part.
pub(crate) fn extract_part_name(part: ExtractPart) -> &'static str {
    EXTRACT_PARTS
        .into_iter()
        .find_map(|(name, candidate)| (candidate == part).then_some(name))
        .unwrap_or_default()
}

const DATE_UNITS: [(&str, DateUnit); 8] = [
    ("year", DateUnit::Year),
    ("quarter", DateUnit::Quarter),
    ("month", DateUnit::Month),
    ("week", DateUnit::Week),
    ("day", DateUnit::Day),
    ("hour", DateUnit::Hour),
    ("minute", DateUnit::Minute),
    ("second", DateUnit::Second),
];

/// The unit of `DATE_TRUNC`, `DATEADD` and `DATEDIFF` named `word`, ignoring case.
pub(crate) fn date_unit(word: &str) -> Option<DateUnit> {
    DATE_UNITS
        .into_iter()
        .find_map(|(name, unit)| is_keyword(word, name).then_some(unit))
}

/// The name [`crate::print`] writes for a date unit.
pub(crate) fn date_unit_name(unit: DateUnit) -> &'static str {
    DATE_UNITS
        .into_iter()
        .find_map(|(name, candidate)| (candidate == unit).then_some(name))
        .unwrap_or_default()
}

/// The `CAST` target types written as one word, without parameters (E0F-11).
const SIMPLE_CAST_TARGETS: [(&str, CastTarget); 12] = [
    ("VARCHAR", CastTarget::Text),
    ("STRING", CastTarget::Text),
    ("INTEGER", CastTarget::Integer),
    ("INT", CastTarget::Integer),
    ("BIGINT", CastTarget::BigInteger),
    ("DOUBLE", CastTarget::Double),
    ("FLOAT", CastTarget::Double),
    ("BOOLEAN", CastTarget::Boolean),
    ("DATE", CastTarget::Date),
    ("TIME", CastTarget::Time),
    ("TIMESTAMP", CastTarget::Timestamp),
    ("TIMESTAMP_NTZ", CastTarget::Timestamp),
];

/// The one-word `CAST` target type named `word`, if E0F-11 lists it.
pub(crate) fn simple_cast_target(word: &str) -> Option<CastTarget> {
    SIMPLE_CAST_TARGETS
        .into_iter()
        .find_map(|(name, target)| is_keyword(word, name).then_some(target))
}

/// The name [`crate::print`] writes for a one-word `CAST` target type; `None` for `DECIMAL`.
pub(crate) fn simple_cast_target_name(target: CastTarget) -> Option<&'static str> {
    SIMPLE_CAST_TARGETS
        .into_iter()
        .find_map(|(name, candidate)| (candidate == target).then_some(name))
}

/// Whether `word` names the decimal target type (`DECIMAL` or `NUMERIC`).
pub(crate) fn is_decimal_type(word: &str) -> bool {
    is_keyword(word, "DECIMAL") || is_keyword(word, "NUMERIC")
}

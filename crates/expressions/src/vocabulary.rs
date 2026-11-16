//! The words of the accepted subset (E0F-2.2): reserved words, functions, aggregates and
//! `CAST` target types.
//!
//! The function, aggregate and type tables are provisional until issue #51 settles the
//! `OSSIE_SQL_2026` lists; each is one table here, so that decision changes one place.

use semantic_compiler_model::expression::{CastTarget, ScalarFunction, SimpleAggregate};

/// Words that are never a regular identifier: a reference must write them as delimited
/// identifiers (`"end"`).
const RESERVED_WORDS: [&str; 32] = [
    "ALL",
    "AND",
    "ANY",
    "AS",
    "BETWEEN",
    "BY",
    "CASE",
    "CAST",
    "CURRENT_DATE",
    "CURRENT_TIMESTAMP",
    "DISTINCT",
    "ELSE",
    "END",
    "ESCAPE",
    "EXISTS",
    "FALSE",
    "FROM",
    "ILIKE",
    "IN",
    "INTERVAL",
    "IS",
    "LIKE",
    "NOT",
    "NULL",
    "OR",
    "ORDER",
    "SELECT",
    "SOME",
    "THEN",
    "TRUE",
    "WHEN",
    "WHERE",
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

/// The scalar function named `word`, if it is in the subset.
pub(crate) fn scalar_function(word: &str) -> Option<ScalarFunction> {
    [
        ("COALESCE", ScalarFunction::Coalesce),
        ("NULLIF", ScalarFunction::Nullif),
        ("ROUND", ScalarFunction::Round),
    ]
    .into_iter()
    .find_map(|(name, function)| is_keyword(word, name).then_some(function))
}

/// The name the specification writes for a scalar function.
pub(crate) fn scalar_function_name(function: ScalarFunction) -> &'static str {
    match function {
        ScalarFunction::Coalesce => "COALESCE",
        ScalarFunction::Nullif => "NULLIF",
        ScalarFunction::Round => "ROUND",
    }
}

/// Whether a scalar function takes `count` arguments.
pub(crate) fn accepts_argument_count(function: ScalarFunction, count: usize) -> bool {
    match function {
        ScalarFunction::Coalesce => count >= 2,
        ScalarFunction::Nullif | ScalarFunction::Round => count == 2,
    }
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

/// The single-argument aggregate named `word`, if it is in the subset.
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

/// The `CAST` target types written as one word, without parameters.
const SIMPLE_CAST_TARGETS: [(&str, CastTarget); 9] = [
    ("BOOLEAN", CastTarget::Boolean),
    ("SMALLINT", CastTarget::SmallInteger),
    ("INTEGER", CastTarget::Integer),
    ("BIGINT", CastTarget::BigInteger),
    ("REAL", CastTarget::Real),
    ("VARCHAR", CastTarget::Varchar),
    ("DATE", CastTarget::Date),
    ("TIME", CastTarget::Time),
    ("TIMESTAMP", CastTarget::Timestamp),
];

/// The one-word `CAST` target type named `word`, if it is in the subset.
pub(crate) fn simple_cast_target(word: &str) -> Option<CastTarget> {
    SIMPLE_CAST_TARGETS
        .into_iter()
        .find_map(|(name, target)| is_keyword(word, name).then_some(target))
}

/// The name of a one-word `CAST` target type, or `None` for `DECIMAL` and `DOUBLE PRECISION`.
pub(crate) fn simple_cast_target_name(target: CastTarget) -> Option<&'static str> {
    SIMPLE_CAST_TARGETS
        .into_iter()
        .find_map(|(name, candidate)| (candidate == target).then_some(name))
}

/// Whether `word` names the decimal target type (`DECIMAL` or `NUMERIC`).
pub(crate) fn is_decimal_type(word: &str) -> bool {
    is_keyword(word, "DECIMAL") || is_keyword(word, "NUMERIC")
}

//! The words the JSON form uses for operators, functions, parts, units and cast targets: one
//! fixed lowercase word per variant, so spelling synonyms share a word (E0H-1.3).

use super::super::call::{
    DateUnit, DistinctAggregate, ExtractPart, PercentileFunction, ScalarFunction, SimpleAggregate,
    SortDirection,
};
use super::super::cast::CastTarget;
use super::super::operator::{BinaryOperator, LikeCase, UnaryOperator};

pub(super) fn unary(operator: UnaryOperator) -> &'static str {
    match operator {
        UnaryOperator::Plus => "plus",
        UnaryOperator::Minus => "minus",
        UnaryOperator::Not => "not",
    }
}

pub(super) fn binary(operator: BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::Add => "add",
        BinaryOperator::Subtract => "subtract",
        BinaryOperator::Multiply => "multiply",
        BinaryOperator::Divide => "divide",
        BinaryOperator::Modulo => "modulo",
        BinaryOperator::Concatenate => "concatenate",
        BinaryOperator::Equal => "equal",
        BinaryOperator::NotEqual => "not_equal",
        BinaryOperator::Less => "less",
        BinaryOperator::LessOrEqual => "less_or_equal",
        BinaryOperator::Greater => "greater",
        BinaryOperator::GreaterOrEqual => "greater_or_equal",
        BinaryOperator::And => "and",
        BinaryOperator::Or => "or",
    }
}

pub(super) fn like_case(case: LikeCase) -> &'static str {
    match case {
        LikeCase::Sensitive => "like",
        LikeCase::Insensitive => "ilike",
    }
}

pub(super) fn scalar_function(function: ScalarFunction) -> &'static str {
    match function {
        ScalarFunction::Year => "year",
        ScalarFunction::Quarter => "quarter",
        ScalarFunction::Month => "month",
        ScalarFunction::Day => "day",
        ScalarFunction::DayOfYear => "day_of_year",
        ScalarFunction::Hour => "hour",
        ScalarFunction::Minute => "minute",
        ScalarFunction::Second => "second",
        ScalarFunction::ToDate => "to_date",
        ScalarFunction::ToTimestamp => "to_timestamp",
        ScalarFunction::Concat => "concat",
        ScalarFunction::Length => "length",
        ScalarFunction::Lower => "lower",
        ScalarFunction::Upper => "upper",
        ScalarFunction::Trim => "trim",
        ScalarFunction::LeftTrim => "left_trim",
        ScalarFunction::RightTrim => "right_trim",
        ScalarFunction::Left => "left",
        ScalarFunction::Right => "right",
        ScalarFunction::Substring => "substring",
        ScalarFunction::Replace => "replace",
        ScalarFunction::SplitPart => "split_part",
        ScalarFunction::Position => "position",
        ScalarFunction::Contains => "contains",
        ScalarFunction::StartsWith => "starts_with",
        ScalarFunction::EndsWith => "ends_with",
        ScalarFunction::Absolute => "absolute",
        ScalarFunction::Sign => "sign",
        ScalarFunction::Floor => "floor",
        ScalarFunction::Ceiling => "ceiling",
        ScalarFunction::Round => "round",
        ScalarFunction::Truncate => "truncate",
        ScalarFunction::Modulo => "modulo",
        ScalarFunction::Power => "power",
        ScalarFunction::SquareRoot => "square_root",
        ScalarFunction::Exponential => "exponential",
        ScalarFunction::NaturalLogarithm => "natural_logarithm",
        ScalarFunction::Logarithm => "logarithm",
        ScalarFunction::DecimalLogarithm => "decimal_logarithm",
        ScalarFunction::Greatest => "greatest",
        ScalarFunction::Least => "least",
        ScalarFunction::If => "if",
        ScalarFunction::Nullif => "nullif",
        ScalarFunction::Coalesce => "coalesce",
        ScalarFunction::IfNull => "if_null",
        ScalarFunction::Nvl2 => "nvl2",
        ScalarFunction::ZeroIfNull => "zero_if_null",
        ScalarFunction::NullIfZero => "null_if_zero",
    }
}

pub(super) fn extract_part(part: ExtractPart) -> &'static str {
    match part {
        ExtractPart::Year => "year",
        ExtractPart::Quarter => "quarter",
        ExtractPart::Month => "month",
        ExtractPart::Week => "week",
        ExtractPart::Day => "day",
        ExtractPart::DayOfWeek => "day_of_week",
        ExtractPart::DayOfYear => "day_of_year",
        ExtractPart::Hour => "hour",
        ExtractPart::Minute => "minute",
        ExtractPart::Second => "second",
    }
}

pub(super) fn date_unit(unit: DateUnit) -> &'static str {
    match unit {
        DateUnit::Year => "year",
        DateUnit::Quarter => "quarter",
        DateUnit::Month => "month",
        DateUnit::Week => "week",
        DateUnit::Day => "day",
        DateUnit::Hour => "hour",
        DateUnit::Minute => "minute",
        DateUnit::Second => "second",
    }
}

pub(super) fn simple_aggregate(function: SimpleAggregate) -> &'static str {
    match function {
        SimpleAggregate::Sum => "sum",
        SimpleAggregate::Count => "count",
        SimpleAggregate::Average => "average",
        SimpleAggregate::Minimum => "minimum",
        SimpleAggregate::Maximum => "maximum",
        SimpleAggregate::StandardDeviation => "standard_deviation",
        SimpleAggregate::PopulationStandardDeviation => "population_standard_deviation",
        SimpleAggregate::SampleStandardDeviation => "sample_standard_deviation",
        SimpleAggregate::Variance => "variance",
        SimpleAggregate::PopulationVariance => "population_variance",
        SimpleAggregate::SampleVariance => "sample_variance",
        SimpleAggregate::Median => "median",
    }
}

pub(super) fn distinct_aggregate(function: DistinctAggregate) -> &'static str {
    match function {
        DistinctAggregate::Sum => "sum",
        DistinctAggregate::Count => "count",
    }
}

pub(super) fn percentile(function: PercentileFunction) -> &'static str {
    match function {
        PercentileFunction::Continuous => "percentile_continuous",
        PercentileFunction::Discrete => "percentile_discrete",
    }
}

pub(super) fn direction(direction: SortDirection) -> &'static str {
    match direction {
        SortDirection::Ascending => "ascending",
        SortDirection::Descending => "descending",
    }
}

/// The cast target word; a decimal target is completed with its precision and scale.
pub(super) fn cast_target(target: CastTarget) -> &'static str {
    match target {
        CastTarget::Text => "text",
        CastTarget::Integer => "integer",
        CastTarget::BigInteger => "big_integer",
        CastTarget::Decimal(_) => "decimal",
        CastTarget::Double => "double",
        CastTarget::Boolean => "boolean",
        CastTarget::Date => "date",
        CastTarget::Time => "time",
        CastTarget::Timestamp => "timestamp",
    }
}

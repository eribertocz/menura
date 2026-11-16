//! E0F-10 functions and E0F-11 `CAST` target types; DEV-10.

// `cfg(test)` marks the helpers as test code, where `expect` is allowed (clippy.toml).
#[cfg(test)]
mod support;

use semantic_compiler_expressions::{ExpressionCode, ExpressionErrorKind, parse};
use semantic_compiler_model::expression::{
    CastTarget, DateUnit, DecimalTarget, Expression, ExtractPart, ScalarFunction,
};
use support::{assert_code, assert_parses, call, column, integer, text, tree};

/// Every plain function of E0F-10 with an accepted argument count.
const PLAIN_CALLS: [&str; 52] = [
    "YEAR(d)",
    "QUARTER(d)",
    "MONTH(d)",
    "DAY(d)",
    "DAYOFYEAR(d)",
    "HOUR(t)",
    "MINUTE(t)",
    "SECOND(t)",
    "TO_DATE(s)",
    "TO_TIMESTAMP(s)",
    "CONCAT(s)",
    "CONCAT(s, t, u)",
    "LENGTH(s)",
    "LOWER(s)",
    "UPPER(s)",
    "TRIM(s)",
    "LTRIM(s)",
    "RTRIM(s)",
    "LEFT(s, 2)",
    "RIGHT(s, 2)",
    "SUBSTRING(s, 1)",
    "SUBSTRING(s, 1, 2)",
    "REPLACE(s, 'a', 'b')",
    "SPLIT_PART(s, ',', 1)",
    "CHARINDEX(a, s)",
    "CONTAINS(s, a)",
    "STARTSWITH(s, a)",
    "ENDSWITH(s, a)",
    "ABS(x)",
    "SIGN(x)",
    "FLOOR(x)",
    "CEIL(x)",
    "CEILING(x)",
    "ROUND(x)",
    "ROUND(x, 2)",
    "TRUNC(x)",
    "TRUNCATE(x, 1)",
    "MOD(x, y)",
    "POWER(x, y)",
    "SQRT(x)",
    "EXP(x)",
    "LN(x)",
    "LOG(2, x)",
    "LOG10(x)",
    "GREATEST(a, b)",
    "LEAST(a, b, c)",
    "IF(c, a, b)",
    "IFF(c, a, b)",
    "IFNULL(a, b)",
    "NVL(a, b)",
    "NVL2(a, b, c)",
    "ZEROIFNULL(x)",
];

#[test]
fn e0f_10_every_listed_function_is_accepted() {
    assert_parses(&PLAIN_CALLS);
    assert_parses(&["NULLIFZERO(x)", "NULLIF(a, b)", "COALESCE(a, b, c)"]);
}

#[test]
fn e0f_10_arities_are_checked() {
    for text in [
        "YEAR()",
        "CONCAT()",
        "LEFT(s)",
        "SUBSTRING(s)",
        "SUBSTRING(s, 1, 2, 3)",
        "ROUND(x, 1, 2)",
        "LOG(x)",
        "GREATEST(a)",
        "IF(c, a)",
        "NVL2(a, b)",
        "DATEADD(day, 1)",
        "DATE_PART('year')",
    ] {
        let error = parse(text).unwrap_err();
        assert!(
            matches!(error.kind(), ExpressionErrorKind::WrongArgumentCount { .. }),
            "{text}: {error}"
        );
        assert_eq!(error.code(), ExpressionCode::OutsideSubset);
    }
}

#[test]
fn e0f_10_synonyms_share_one_tree() {
    for (left, right) in [
        ("CEIL(x)", "CEILING(x)"),
        ("TRUNC(x)", "TRUNCATE(x)"),
        ("IF(c, a, b)", "IFF(c, a, b)"),
        ("IFNULL(a, b)", "NVL(a, b)"),
        ("POSITION('a' IN s)", "CHARINDEX('a', s)"),
        ("EXTRACT(YEAR FROM d)", "DATE_PART('year', d)"),
        ("DATEADD(month, 1, d)", "DATEADD('MONTH', 1, d)"),
    ] {
        assert_eq!(tree(left), tree(right), "{left} and {right}");
    }
    assert_eq!(
        tree("position('a' in s)"),
        call(ScalarFunction::Position, vec![text("a"), column("s")])
    );
}

#[test]
fn e0f_10_1_date_parts_are_closed_enums() {
    assert_eq!(
        tree("EXTRACT(dayofweek FROM d)"),
        Expression::Extract {
            part: ExtractPart::DayOfWeek,
            operand: Box::new(column("d")),
        }
    );
    assert_eq!(
        tree("DATE_TRUNC('week', d)"),
        Expression::DateTrunc {
            unit: DateUnit::Week,
            operand: Box::new(column("d")),
        }
    );
    assert_eq!(
        tree("DATEDIFF('day', a, b)"),
        Expression::DateDiff {
            unit: DateUnit::Day,
            start: Box::new(column("a")),
            end: Box::new(column("b")),
        }
    );
    assert_eq!(
        tree("DATEADD(quarter, 2, d)"),
        Expression::DateAdd {
            unit: DateUnit::Quarter,
            amount: Box::new(integer("2")),
            operand: Box::new(column("d")),
        }
    );
    assert_code(
        ExpressionCode::OutsideSubset,
        &[
            "EXTRACT(FORTNIGHT FROM d)",
            "DATE_TRUNC(month, d)",
            "DATE_TRUNC('dayofweek', d)",
            "DATE_PART(year, d)",
            "DATEADD(\"day\", 1, d)",
            "DATEDIFF(t.day, a, b)",
        ],
    );
    assert_code(
        ExpressionCode::Unparsable,
        &["EXTRACT('year' FROM d)", "EXTRACT(YEAR d)"],
    );
}

#[test]
fn e0f_11_cast_targets() {
    let cast = |target| Expression::Cast {
        operand: Box::new(column("a")),
        target,
    };
    for (text, target) in [
        ("CAST(a AS VARCHAR)", CastTarget::Text),
        ("CAST(a AS string)", CastTarget::Text),
        ("CAST(a AS INT)", CastTarget::Integer),
        ("CAST(a AS INTEGER)", CastTarget::Integer),
        ("CAST(a AS BIGINT)", CastTarget::BigInteger),
        ("CAST(a AS FLOAT)", CastTarget::Double),
        ("CAST(a AS DOUBLE)", CastTarget::Double),
        ("CAST(a AS BOOLEAN)", CastTarget::Boolean),
        ("CAST(a AS DATE)", CastTarget::Date),
        ("CAST(a AS TIME)", CastTarget::Time),
        ("CAST(a AS TIMESTAMP)", CastTarget::Timestamp),
        ("CAST(a AS TIMESTAMP_NTZ)", CastTarget::Timestamp),
        (
            "CAST(a AS NUMERIC(18, 2))",
            CastTarget::Decimal(DecimalTarget::new(18, 2).unwrap()),
        ),
        (
            "CAST(a AS DECIMAL(10))",
            CastTarget::Decimal(DecimalTarget::new(10, 0).unwrap()),
        ),
    ] {
        assert_eq!(tree(text), cast(target), "{text}");
    }
}

#[test]
fn e0f_11_1_other_targets_are_outside_the_subset() {
    assert_code(
        ExpressionCode::OutsideSubset,
        &[
            "CAST(a AS DOUBLE PRECISION)",
            "CAST(a AS REAL)",
            "CAST(a AS SMALLINT)",
            "CAST(a AS JSON)",
            "CAST(a AS DECIMAL)",
            "CAST(a AS DECIMAL(39, 2))",
            "CAST(a AS DECIMAL(2, 3))",
            "CAST(a AS VARCHAR(10))",
            "CAST(a AS TIMESTAMP(3))",
            "CAST(a AS TIMESTAMP WITH TIME ZONE)",
        ],
    );
}

#[test]
fn dev_10_regexp_like_and_millisecond_are_left_out() {
    assert_code(
        ExpressionCode::OutsideSubset,
        &[
            "REGEXP_LIKE(s, '^a')",
            "EXTRACT(MILLISECOND FROM t)",
            "DATE_PART('millisecond', t)",
        ],
    );
}

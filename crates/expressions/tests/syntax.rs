//! E0F-9: the grammar decides what parses, then the subset decides what is accepted.

// `cfg(test)` marks the helpers as test code, where `expect` is allowed (clippy.toml).
#[cfg(test)]
mod support;

use semantic_compiler_expressions::ExpressionCode;
use semantic_compiler_model::expression::{
    BinaryOperator, DateValue, Expression, FloatLiteral, IntegerLiteral, Literal, TimeOfDay,
    TimestampValue, UnaryOperator,
};
use support::{assert_code, assert_parses, binary, column, decimal, integer, text, tree};

#[test]
fn e0f_9_1_texts_the_grammar_rejects_are_e0f_01() {
    assert_code(
        ExpressionCode::Unparsable,
        &[
            "a LIKE 'x' ESCAPE '!'",
            "a BETWEEN SYMMETRIC 1 AND 2",
            "a IS TRUE",
            "a IS NOT UNKNOWN",
            "a = ANY (b)",
            "a > ALL (b)",
            "INTERVAL '1' DAY",
            "SUM(ALL a)",
            "a::INTEGER",
        ],
    );
}

#[test]
fn e0f_9_1_calls_outside_the_subset_are_parsed_before_they_are_rejected() {
    assert_code(
        ExpressionCode::Unparsable,
        &[
            "DATE_ADD(d, INTERVAL 7 DAY)",
            "FOO(a b)",
            "TRY_CAST(x AS)",
            "FOO(a) WITHIN GROUP (a)",
            "FOO(a) FILTER (a)",
        ],
    );
    assert_code(
        ExpressionCode::OutsideSubset,
        &[
            "FOO(DISTINCT a) WITHIN GROUP (ORDER BY a, b DESC) FILTER (WHERE c)",
            "TRY_CAST(x AS DECIMAL(10, 2))",
            "FOO(*)",
        ],
    );
}

#[test]
fn e0f_9_1_non_reserved_words_are_identifiers() {
    assert_parses(&[
        "date",
        "first",
        "date + 1",
        "CASE expression WHEN 1 THEN first END",
    ]);
    assert_eq!(tree("first"), column("first"));
}

#[test]
fn e0f_9_2_precedence() {
    let expected = binary(
        BinaryOperator::Or,
        binary(BinaryOperator::Equal, column("a"), integer("1")),
        binary(
            BinaryOperator::And,
            binary(
                BinaryOperator::Greater,
                binary(
                    BinaryOperator::Add,
                    column("b"),
                    binary(BinaryOperator::Modulo, column("c"), integer("2")),
                ),
                integer("3"),
            ),
            Expression::Unary {
                operator: UnaryOperator::Not,
                operand: Box::new(Expression::IsNull {
                    operand: Box::new(column("d")),
                    negated: false,
                }),
            },
        ),
    );
    assert_eq!(tree("a = 1 OR b + c % 2 > 3 AND NOT d IS NULL"), expected);
    assert_eq!(
        tree("-a * b"),
        binary(
            BinaryOperator::Multiply,
            Expression::Unary {
                operator: UnaryOperator::Minus,
                operand: Box::new(column("a")),
            },
            column("b"),
        )
    );
}

#[test]
fn e0f_9_2_additive_and_concatenation_share_one_level_left_to_right() {
    assert_eq!(
        tree("a || b + c"),
        binary(
            BinaryOperator::Add,
            binary(BinaryOperator::Concatenate, column("a"), column("b")),
            column("c"),
        )
    );
    assert_eq!(
        tree("a - b - c"),
        binary(
            BinaryOperator::Subtract,
            binary(BinaryOperator::Subtract, column("a"), column("b")),
            column("c"),
        )
    );
    assert_code(
        ExpressionCode::Unparsable,
        &["a < b < c", "a IS NULL IS NULL"],
    );
}

#[test]
fn e0f_9_3_literals() {
    assert_eq!(tree("'it''s'"), text("it's"));
    assert_eq!(tree("42"), integer("42"));
    assert_eq!(tree("0.50"), decimal("0", "50"));
    assert_eq!(tree(".5"), decimal("", "5"));
    assert_eq!(tree("10."), decimal("10", ""));
    assert_eq!(
        tree("1.5E-3"),
        Expression::Literal(Literal::Float(FloatLiteral::new("1", "5", -3).unwrap()))
    );
    assert_eq!(tree("TRUE"), Expression::Literal(Literal::Boolean(true)));
    assert_eq!(tree("NULL"), Expression::Literal(Literal::Null));
    assert_eq!(
        tree("DATE '2024-02-29'"),
        Expression::Literal(Literal::Date(DateValue::new(2024, 2, 29).unwrap()))
    );
    assert_eq!(
        tree("TIME '08:30:00.123456'"),
        Expression::Literal(Literal::Time(TimeOfDay::new(8, 30, 0, "123456").unwrap()))
    );
    let timestamp = Expression::Literal(Literal::Timestamp(TimestampValue {
        date: DateValue::new(2026, 11, 15).unwrap(),
        time: TimeOfDay::new(8, 30, 0, "25").unwrap(),
    }));
    assert_eq!(tree("TIMESTAMP '2026-11-15 08:30:00.25'"), timestamp);
    assert_eq!(tree("timestamp_ntz '2026-11-15 08:30:00.25'"), timestamp);
}

#[test]
fn e0f_9_3_exact_numbers_have_at_most_38_digits() {
    let eighteen = "9".repeat(18);
    let thirty_eight = "9".repeat(38);
    assert_eq!(tree(&eighteen), integer(&eighteen));
    assert_eq!(tree(&thirty_eight), integer(&thirty_eight));
    assert_eq!(IntegerLiteral::MAXIMUM_BIG_INTEGER_DIGITS, 18);
    let decimal_text = format!("{}.{}", "1".repeat(20), "2".repeat(18));
    assert_parses(&[&decimal_text, &format!("{}E2", "9".repeat(50))]);
    assert_code(
        ExpressionCode::OutsideSubset,
        &[
            &"9".repeat(39),
            &format!("{}.{}", "1".repeat(20), "2".repeat(19)),
        ],
    );
}

#[test]
fn e0f_9_3_typed_literals_need_their_exact_form() {
    assert_code(
        ExpressionCode::Unparsable,
        &[
            "DATE '2026-02-30'",
            "DATE '0000-01-01'",
            "DATE '2026-1-01'",
            "TIME '24:00:00'",
            "TIME '08:30:00.1234567'",
            "TIME '08:30'",
            "TIMESTAMP '2026-01-01T00:00:00'",
            "TIMESTAMP '2026-01-01 00:00:00.1234567'",
            "TIMESTAMP '2026-01-01 00:00:00+02:00'",
            "TIMESTAMP_NTZ '2026-01-01'",
        ],
    );
}

#[test]
fn e0f_9_4_call_forms_only_where_listed() {
    assert_code(
        ExpressionCode::OutsideSubset,
        &[
            "SUM(*)",
            "UPPER(*)",
            "AVG(DISTINCT a)",
            "UPPER(DISTINCT a)",
            "SUM(DISTINCT a, b)",
            "SUM(a) WITHIN GROUP (ORDER BY a)",
            "PERCENTILE_CONT(0.5)",
            "PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY a, b)",
            "UPPER(a) FILTER (WHERE b)",
            "COUNT(a, b)",
        ],
    );
    assert_parses(&[
        "COUNT(*)",
        "SUM(DISTINCT a) FILTER (WHERE b)",
        "MEDIAN(a) FILTER (WHERE b)",
        "PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY a NULLS FIRST) FILTER (WHERE b)",
    ]);
}

#[test]
fn e0f_9_1_window_clauses_follow_the_grammar() {
    assert_code(
        ExpressionCode::Unparsable,
        &[
            "SUM(a) OVER (ORDER BY)",
            "SUM(a) OVER w",
            "SUM(a) OVER",
            "SUM(a) OVER (PARTITION a)",
            "SUM(a) OVER (ORDER BY a NULLS)",
            "SUM(a) OVER (ROWS)",
            "SUM(a) OVER (ROWS 1)",
            "SUM(a) OVER (ROWS BETWEEN 1 PRECEDING)",
            "SUM(a) OVER (ROWS BETWEEN CURRENT AND 1 FOLLOWING)",
            "SUM(a) OVER (ORDER BY a PARTITION BY b)",
            "ROW_NUMBER() OVER (ORDER BY a",
            "UPPER(a) OVER (GROUPS 1 PRECEDING)",
        ],
    );
    assert_code(
        ExpressionCode::OutsideSubset,
        &[
            "SUM(a) OVER ()",
            "RANK() OVER (ORDER BY revenue DESC NULLS LAST)",
            "SUM(a) OVER (ORDER BY d ROWS UNBOUNDED PRECEDING)",
        ],
    );
}

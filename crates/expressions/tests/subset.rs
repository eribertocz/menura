//! The accepted subset of E0F-2.2 and the codes of E0F-8.

// `cfg(test)` marks the helpers as test code, where `expect` is allowed (clippy.toml).
#[cfg(test)]
mod support;

use semantic_compiler_expressions::{ExpressionCode, ExpressionErrorKind, Span, parse};
use semantic_compiler_model::expression::{
    AggregateCall, ApproximateNumber, BinaryOperator, CaseBranch, CastTarget, DateValue,
    DecimalTarget, DistinctAggregate, ExactNumber, Expression, Identifier, LikeCase, Literal,
    PercentileFunction, Reference, ScalarFunction, SimpleAggregate, SortDirection, TimeOfDay,
    TimestampValue, UnaryOperator,
};
use support::{binary, code_of, column, decimal, field, integer, text, tree};

#[test]
fn e0f_2_2_literals() {
    assert_eq!(tree("'it''s'"), text("it's"));
    assert_eq!(tree("42"), integer("42"));
    assert_eq!(tree("0.50"), decimal("0", "50"));
    assert_eq!(
        tree("1.5E-3"),
        Expression::Literal(Literal::Approximate(ApproximateNumber {
            mantissa: ExactNumber::new("1", "5").unwrap(),
            exponent: -3,
        }))
    );
    assert_eq!(tree("TRUE"), Expression::Literal(Literal::Boolean(true)));
    assert_eq!(tree("false"), Expression::Literal(Literal::Boolean(false)));
    assert_eq!(tree("NULL"), Expression::Literal(Literal::Null));
    assert_eq!(
        tree("DATE '2024-02-29'"),
        Expression::Literal(Literal::Date(DateValue::new(2024, 2, 29).unwrap()))
    );
    assert_eq!(
        tree("TIMESTAMP '2026-11-15 08:30:00.25'"),
        Expression::Literal(Literal::Timestamp(TimestampValue {
            date: DateValue::new(2026, 11, 15).unwrap(),
            time: TimeOfDay::new(8, 30, 0, "25").unwrap(),
        }))
    );
}

#[test]
fn e0f_2_2_references() {
    assert_eq!(tree("amount"), column("amount"));
    assert_eq!(tree("orders.amount"), field("orders", "amount"));
    assert_eq!(
        tree("\"Order \"\"Total\"\"\""),
        Expression::Reference(Reference {
            qualifier: None,
            name: Identifier::delimited("Order \"Total\"").unwrap(),
        })
    );
    assert_eq!(
        tree("AGG(total_amount)"),
        Expression::MetricAggregate(Reference {
            qualifier: None,
            name: Identifier::regular("total_amount").unwrap(),
        })
    );
    assert_eq!(
        tree("USER_ATTRIBUTE('region')"),
        Expression::UserAttribute("region".to_owned())
    );
}

#[test]
fn e0f_2_2_operators_follow_precedence() {
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
}

#[test]
fn e0f_2_2_operators_of_one_level_associate_to_the_left() {
    assert_eq!(
        tree("a - b - c"),
        binary(
            BinaryOperator::Subtract,
            binary(BinaryOperator::Subtract, column("a"), column("b")),
            column("c"),
        )
    );
    assert_eq!(
        tree("a || b + c"),
        binary(
            BinaryOperator::Concatenate,
            column("a"),
            binary(BinaryOperator::Add, column("b"), column("c")),
        )
    );
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
fn e0f_2_2_predicates() {
    assert_eq!(
        tree("a IS NOT DISTINCT FROM b"),
        Expression::IsDistinctFrom {
            left: Box::new(column("a")),
            right: Box::new(column("b")),
            negated: true,
        }
    );
    assert_eq!(
        tree("a NOT IN (1, 2)"),
        Expression::InList {
            operand: Box::new(column("a")),
            list: vec![integer("1"), integer("2")],
            negated: true,
        }
    );
    assert_eq!(
        tree("a BETWEEN 1 AND 2 AND b"),
        binary(
            BinaryOperator::And,
            Expression::Between {
                operand: Box::new(column("a")),
                low: Box::new(integer("1")),
                high: Box::new(integer("2")),
                negated: false,
            },
            column("b"),
        )
    );
    assert_eq!(
        tree("a NOT ILIKE 'x%'"),
        Expression::Like {
            operand: Box::new(column("a")),
            pattern: Box::new(text("x%")),
            case: LikeCase::Insensitive,
            negated: true,
        }
    );
    for (operator, symbol) in [
        (BinaryOperator::NotEqual, "<>"),
        (BinaryOperator::Less, "<"),
        (BinaryOperator::LessOrEqual, "<="),
        (BinaryOperator::GreaterOrEqual, ">="),
    ] {
        assert_eq!(
            tree(&format!("a {symbol} b")),
            binary(operator, column("a"), column("b"))
        );
    }
}

#[test]
fn e0f_2_2_conditionals_and_conversion() {
    assert_eq!(
        tree("CASE a WHEN 1 THEN 'one' END"),
        Expression::Case {
            operand: Some(Box::new(column("a"))),
            branches: vec![CaseBranch {
                condition: integer("1"),
                result: text("one"),
            }],
            otherwise: None,
        }
    );
    assert_eq!(
        tree("COALESCE(a, b, 0)"),
        Expression::Function {
            function: ScalarFunction::Coalesce,
            arguments: vec![column("a"), column("b"), integer("0")],
        }
    );
    assert_eq!(
        tree("NULLIF(a, 0)"),
        Expression::Function {
            function: ScalarFunction::Nullif,
            arguments: vec![column("a"), integer("0")],
        }
    );
    assert_eq!(
        tree("CAST(a AS NUMERIC(18, 2))"),
        Expression::Cast {
            operand: Box::new(column("a")),
            target: CastTarget::Decimal(DecimalTarget::new(18, 2).unwrap()),
        }
    );
    assert_eq!(
        tree("cast(a as double precision)"),
        Expression::Cast {
            operand: Box::new(column("a")),
            target: CastTarget::DoublePrecision,
        }
    );
}

#[test]
fn e0f_2_2_aggregates() {
    assert_eq!(
        tree("COUNT(*) FILTER (WHERE a > 0)"),
        Expression::Aggregate {
            call: AggregateCall::CountRows,
            filter: Some(Box::new(binary(
                BinaryOperator::Greater,
                column("a"),
                integer("0")
            ))),
        }
    );
    assert_eq!(
        tree("count(distinct a)"),
        Expression::Aggregate {
            call: AggregateCall::Distinct {
                function: DistinctAggregate::Count,
                argument: Box::new(column("a")),
            },
            filter: None,
        }
    );
    assert_eq!(
        tree("STDDEV_SAMP(a)"),
        Expression::Aggregate {
            call: AggregateCall::Simple {
                function: SimpleAggregate::SampleStandardDeviation,
                argument: Box::new(column("a")),
            },
            filter: None,
        }
    );
    assert_eq!(
        tree("PERCENTILE_DISC(0.9) WITHIN GROUP (ORDER BY a DESC)"),
        Expression::Aggregate {
            call: AggregateCall::Percentile {
                function: PercentileFunction::Discrete,
                fraction: Box::new(decimal("0", "9")),
                ordering: Box::new(column("a")),
                direction: SortDirection::Descending,
            },
            filter: None,
        }
    );
}

#[test]
fn e0f_2_3_current_date_and_timestamp_are_accepted() {
    assert_eq!(tree("current_date"), Expression::CurrentDate);
    assert_eq!(tree("CURRENT_TIMESTAMP"), Expression::CurrentTimestamp);
}

#[test]
fn e0f_2_3_subqueries_and_other_functions_are_outside_the_subset() {
    for text in [
        "(SELECT 1)",
        "a IN (SELECT b FROM t)",
        "EXISTS (SELECT 1)",
        "RANDOM()",
        "now()",
        "CURRENT_DATE()",
        "list_sum(a)",
        "a::DOUBLE",
        "EXTRACT(YEAR FROM a)",
    ] {
        assert_eq!(code_of(text), ExpressionCode::OutsideSubset, "{text}");
    }
}

#[test]
fn e0f_2_4_window_functions_are_reported_as_such() {
    for text in [
        "SUM(a) OVER (PARTITION BY b)",
        "ROW_NUMBER() OVER (ORDER BY a)",
        "1 + LAG(a, 1) OVER w",
    ] {
        let error = parse(text).unwrap_err();
        assert_eq!(error.kind(), &ExpressionErrorKind::WindowFunction, "{text}");
        assert_eq!(error.code(), ExpressionCode::OutsideSubset);
    }
}

#[test]
fn e0f_2_5_division_and_round_are_kept_as_written() {
    assert_eq!(
        tree("ROUND(a / 2, 0)"),
        Expression::Function {
            function: ScalarFunction::Round,
            arguments: vec![
                binary(BinaryOperator::Divide, column("a"), integer("2")),
                integer("0"),
            ],
        }
    );
}

#[test]
fn e0f_01_unparsable_expressions() {
    for text in [
        "",
        "   ",
        "SUM(a",
        "a +",
        "a b",
        "'open",
        "\"open",
        "\"\"",
        "/* open",
        "a = b = c",
        "1e",
        "1e99999999999",
        "a ? b",
        "DATE '2026-02-30'",
        "TIMESTAMP '2026-01-01T00:00:00'",
        "CASE END",
        "a IN ()",
        "and",
        "a.",
    ] {
        assert_eq!(code_of(text), ExpressionCode::Unparsable, "{text:?}");
    }
}

#[test]
fn e0f_02_constructs_outside_the_subset() {
    for text in [
        "a != b",
        "a LIKE 'x' ESCAPE '!'",
        "a BETWEEN SYMMETRIC 1 AND 2",
        "a IS TRUE",
        "a = ANY (b)",
        "INTERVAL '1' DAY",
        "CAST(a AS JSON)",
        "CAST(a AS DECIMAL(39, 2))",
        "CAST(a AS DECIMAL)",
        "MIN(DISTINCT a)",
        "SUM(ALL a)",
        "NULLIF(a)",
        "COALESCE(a)",
        "ROUND(a)",
        "USER_ATTRIBUTE(a)",
        "catalog.schema.table",
        "PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY a NULLS FIRST)",
        "\"MyFunction\"(a)",
    ] {
        assert_eq!(code_of(text), ExpressionCode::OutsideSubset, "{text:?}");
    }
}

#[test]
fn e0f_8_errors_carry_their_location() {
    let error = parse("a + RANDOM()").unwrap_err();
    assert_eq!(
        error.kind(),
        &ExpressionErrorKind::UnsupportedFunction("RANDOM".to_owned())
    );
    assert_eq!(error.span(), Span { start: 4, end: 10 });
    let error = parse("a + ").unwrap_err();
    assert_eq!(error.span(), Span { start: 4, end: 4 });
    assert_eq!(error.code().as_str(), "E0F-01");
}

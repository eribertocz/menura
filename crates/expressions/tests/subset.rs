//! The accepted subset of E0F-2 and the codes of E0F-8.

// `cfg(test)` marks the helpers as test code, where `expect` is allowed (clippy.toml).
#[cfg(test)]
mod support;

use semantic_compiler_expressions::{ExpressionCode, ExpressionErrorKind, Span, parse};
use semantic_compiler_model::expression::{
    AggregateCall, BinaryOperator, CaseBranch, DistinctAggregate, Expression, Identifier, LikeCase,
    PercentileFunction, Reference, ScalarFunction, SimpleAggregate, SortDirection, UnaryOperator,
};
use support::{assert_code, binary, call, code_of, column, decimal, field, integer, text, tree};

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
        (BinaryOperator::NotEqual, "!="),
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
fn e0f_2_2_conditionals() {
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
        call(
            ScalarFunction::Coalesce,
            vec![column("a"), column("b"), integer("0")]
        )
    );
    assert_eq!(
        tree("NULLIF(a, 0)"),
        call(ScalarFunction::Nullif, vec![column("a"), integer("0")])
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
        tree("PERCENTILE_DISC(0.9) WITHIN GROUP (ORDER BY a DESC NULLS LAST)"),
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
fn e0f_2_3_current_moments_are_accepted() {
    assert_eq!(tree("current_date"), Expression::CurrentDate);
    assert_eq!(tree("CURRENT_DATE()"), Expression::CurrentDate);
    assert_eq!(tree("CURRENT_TIME"), Expression::CurrentTime);
    assert_eq!(tree("CURRENT_TIMESTAMP ( )"), Expression::CurrentTimestamp);
}

#[test]
fn e0f_2_3_subqueries_and_other_functions_are_outside_the_subset() {
    assert_code(
        ExpressionCode::OutsideSubset,
        &[
            "(SELECT 1)",
            "a IN (SELECT b FROM t)",
            "EXISTS (SELECT 1)",
            "RANDOM()",
            "now()",
            "list_sum(a)",
            "a::DOUBLE",
        ],
    );
}

#[test]
fn e0f_2_4_window_functions_are_reported_as_such() {
    for text in [
        "SUM(a) OVER (PARTITION BY b)",
        "SUM(a) FILTER (WHERE b) OVER w",
        "ROW_NUMBER() OVER (ORDER BY a)",
        "1 + LAG(a, 1) OVER w",
        "UPPER(a) OVER ()",
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
        call(
            ScalarFunction::Round,
            vec![
                binary(BinaryOperator::Divide, column("a"), integer("2")),
                integer("0"),
            ],
        )
    );
}

#[test]
fn e0f_2_6_everything_else_is_outside_the_subset() {
    assert_code(
        ExpressionCode::OutsideSubset,
        &[
            "TRY_CAST(a AS INTEGER)",
            "REGEXP_LIKE(a, 'x')",
            "EXTRACT(MILLISECOND FROM a)",
            "DATE_PART('millisecond', a)",
            "SOUNDEX(a)",
        ],
    );
}

#[test]
fn e0f_01_unparsable_expressions() {
    assert_code(
        ExpressionCode::Unparsable,
        &[
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
            "CASE END",
            "a IN ()",
            "and",
            "a.",
        ],
    );
}

#[test]
fn e0f_02_constructs_outside_the_subset() {
    assert_code(
        ExpressionCode::OutsideSubset,
        &[
            "CAST(a AS JSON)",
            "MIN(DISTINCT a)",
            "NULLIF(a)",
            "COALESCE(a)",
            "USER_ATTRIBUTE(a)",
            "catalog.schema.table",
            "\"MyFunction\"(a)",
        ],
    );
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
    assert_eq!(code_of("a +").as_str(), "E0F-01");
    let error = parse("x + DATE_TRUNC('fortnight', d)").unwrap_err();
    assert_eq!(error.span(), Span { start: 4, end: 30 });
    let unary = tree("-a");
    assert_eq!(
        unary,
        Expression::Unary {
            operator: UnaryOperator::Minus,
            operand: Box::new(column("a")),
        }
    );
}

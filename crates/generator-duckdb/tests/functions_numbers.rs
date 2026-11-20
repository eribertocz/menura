//! Unit tests of the numeric functions (E0F-10.3) and the conditional functions (E0F-10.4),
//! with the result types of E0F-5a.

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::{FloatBits, IntegerBits, LogicalType};
use support::{big_integer, decimal, expression_sql, from_model, query_with_reference_time};

/// The SQL of `expression`, a model expression of type `logical_type`.
#[cfg(test)]
fn model_sql(expression: &str, logical_type: LogicalType) -> String {
    expression_sql(
        expression,
        logical_type,
        from_model("/semantic_model/0/datasets/0/fields/0"),
        &query_with_reference_time(),
    )
    .expect("SQL")
    .sql
}

fn huge_integer() -> LogicalType {
    LogicalType::Integer {
        bits: IntegerBits::OneHundredTwentyEight,
        signed: true,
    }
}

fn double() -> LogicalType {
    LogicalType::Float {
        bits: FloatBits::SixtyFour,
    }
}

#[test]
fn e0f_10_3_absolute_value_widens_integers() {
    assert!(
        model_sql("ABS(QUANTITY)", huge_integer())
            .contains(r#"CAST(abs(CAST("QUANTITY" AS HUGEINT)) AS HUGEINT)"#)
    );
    assert!(
        model_sql("ABS(AMOUNT)", decimal(10, 2))
            .contains(r#"CAST(abs("AMOUNT") AS DECIMAL(10,2))"#)
    );
}

#[test]
fn e0f_10_3_sign_is_a_64_bit_integer() {
    assert!(model_sql("SIGN(AMOUNT)", big_integer()).contains(r#"CAST(sign("AMOUNT") AS BIGINT)"#));
}

#[test]
fn e0f_10_3_floor_and_ceiling_types() {
    assert!(
        model_sql("FLOOR(AMOUNT)", decimal(9, 0))
            .contains(r#"CAST(floor("AMOUNT") AS DECIMAL(9,0))"#)
    );
    assert!(
        model_sql("CEILING(QUANTITY)", huge_integer())
            .contains(r#"CAST(ceil("QUANTITY") AS HUGEINT)"#)
    );
}

#[test]
fn e0f_10_3_round_a_decimal_to_its_literal_scale() {
    assert!(
        model_sql("ROUND(AMOUNT, 1)", decimal(10, 1))
            .contains(r#"CAST(round("AMOUNT", 1) AS DECIMAL(10,1))"#)
    );
    assert!(
        model_sql("ROUND(AMOUNT)", decimal(10, 0))
            .contains(r#"CAST(round("AMOUNT", 0) AS DECIMAL(10,0))"#)
    );
}

#[test]
fn e0f_10_3_round_an_integer_to_tens() {
    assert!(
        model_sql("ROUND(QUANTITY, -1)", huge_integer())
            .contains(r#"CAST(CAST("QUANTITY" AS HUGEINT) - CAST("QUANTITY" AS HUGEINT) % CAST('10' AS HUGEINT) + CASE WHEN abs(CAST("QUANTITY" AS HUGEINT) % CAST('10' AS HUGEINT)) >= CAST('10' AS HUGEINT) - abs(CAST("QUANTITY" AS HUGEINT) % CAST('10' AS HUGEINT)) THEN sign(CAST("QUANTITY" AS HUGEINT)) * CAST('10' AS HUGEINT) ELSE 0 END AS HUGEINT)"#)
    );
    assert!(
        model_sql("ROUND(QUANTITY)", huge_integer()).contains(r#"CAST("QUANTITY" AS HUGEINT)"#)
    );
}

#[test]
fn e0f_10_3_round_a_float() {
    assert!(model_sql("ROUND(QUANTITY / 2, 0)", double()).contains(
        r#"CAST(round(CASE WHEN CAST(2 AS DOUBLE) = 0 THEN NULL ELSE CAST("QUANTITY" AS DOUBLE) / CAST(2 AS DOUBLE) END, 0) AS DOUBLE)"#
    ));
}

#[test]
fn e0f_10_3_truncate_toward_zero() {
    assert!(model_sql("TRUNC(QUANTITY, -2)", huge_integer()).contains(
        r#"CAST(CAST("QUANTITY" AS HUGEINT) - CAST("QUANTITY" AS HUGEINT) % CAST('100' AS HUGEINT) AS HUGEINT)"#
    ));
    assert!(model_sql("TRUNCATE(AMOUNT, 1)", decimal(10, 1)).contains(
        r#"CAST(round("AMOUNT" - sign("AMOUNT") * CAST('0.05' AS DECIMAL(2,2)), 1) AS DECIMAL(10,1))"#
    ));
    assert!(
        model_sql("TRUNC(AMOUNT)", decimal(10, 0))
            .contains(r#"CAST(round("AMOUNT" - sign("AMOUNT") * CAST('0.5' AS DECIMAL(1,1)), 0) AS DECIMAL(10,0))"#)
    );
    assert!(
        model_sql("TRUNC(CAST(AMOUNT AS DOUBLE), 2)", double())
            .contains(r#"CAST(trunc(CAST("AMOUNT" AS DOUBLE) * 1e2) / 1e2 AS DOUBLE)"#)
    );
}

#[test]
fn e0f_10_3_modulo_by_zero_is_null() {
    assert!(model_sql("MOD(QUANTITY, 3)", huge_integer()).contains(
        r#"CASE WHEN 3 = 0 THEN NULL ELSE CAST("QUANTITY" AS HUGEINT) % CAST(3 AS HUGEINT) END"#
    ));
}

#[test]
fn e0f_10_3_outside_the_domain_is_null() {
    assert!(model_sql("SQRT(AMOUNT)", double()).contains(
        r#"CAST(CASE WHEN "AMOUNT" < 0 THEN NULL ELSE sqrt(CAST("AMOUNT" AS DOUBLE)) END AS DOUBLE)"#
    ));
    assert!(
        model_sql("LN(AMOUNT)", double())
            .contains(r#"CASE WHEN "AMOUNT" <= 0 THEN NULL ELSE ln(CAST("AMOUNT" AS DOUBLE)) END"#)
    );
    assert!(
        model_sql("LOG10(AMOUNT)", double()).contains(
            r#"CASE WHEN "AMOUNT" <= 0 THEN NULL ELSE log10(CAST("AMOUNT" AS DOUBLE)) END"#
        )
    );
    assert!(model_sql("LOG(2, AMOUNT)", double()).contains(
        r#"CASE WHEN 2 <= 0 OR 2 = 1 OR "AMOUNT" <= 0 THEN NULL ELSE ln(CAST("AMOUNT" AS DOUBLE)) / ln(CAST(2 AS DOUBLE)) END"#
    ));
    assert!(model_sql("POWER(AMOUNT, QUANTITY)", double()).contains(
        r#"CASE WHEN ("AMOUNT" < 0 AND "QUANTITY" <> floor("QUANTITY")) OR ("AMOUNT" = 0 AND "QUANTITY" < 0) THEN NULL ELSE pow(CAST("AMOUNT" AS DOUBLE), CAST("QUANTITY" AS DOUBLE)) END"#
    ));
    assert!(
        model_sql("EXP(AMOUNT)", double())
            .contains(r#"CAST(exp(CAST("AMOUNT" AS DOUBLE)) AS DOUBLE)"#)
    );
}

#[test]
fn e0f_10_3_greatest_and_least_are_null_when_an_argument_is_null() {
    assert!(model_sql("GREATEST(AMOUNT, -AMOUNT)", decimal(10, 2)).contains(
        r#"CAST(CASE WHEN "AMOUNT" IS NULL OR (-"AMOUNT") IS NULL THEN NULL ELSE greatest(CAST("AMOUNT" AS DECIMAL(10,2)), CAST((-"AMOUNT") AS DECIMAL(10,2))) END AS DECIMAL(10,2))"#
    ));
    assert!(
        model_sql("LEAST(QUANTITY, 3)", big_integer())
            .contains(r#"ELSE least(CAST("QUANTITY" AS BIGINT), CAST(3 AS BIGINT)) END"#)
    );
}

#[test]
fn e0f_10_4_if_is_the_else_branch_when_the_condition_is_null() {
    assert!(model_sql("IF(QUANTITY > 1, QUANTITY, 0)", big_integer()).contains(
        r#"CAST(CASE WHEN "QUANTITY" > 1 THEN CAST("QUANTITY" AS BIGINT) ELSE CAST(0 AS BIGINT) END AS BIGINT)"#
    ));
}

#[test]
fn e0f_10_4_null_handling_functions() {
    assert!(
        model_sql("COALESCE(QUANTITY, 0)", big_integer())
            .contains(r#"CAST(coalesce(CAST("QUANTITY" AS BIGINT), CAST(0 AS BIGINT)) AS BIGINT)"#)
    );
    assert!(model_sql("IFNULL(QUANTITY, 0)", big_integer()).contains("coalesce("));
    assert!(model_sql("NVL2(REGION, QUANTITY, 0)", big_integer()).contains(
        r#"CASE WHEN "REGION" IS NOT NULL THEN CAST("QUANTITY" AS BIGINT) ELSE CAST(0 AS BIGINT) END"#
    ));
    assert!(
        model_sql("NULLIF(QUANTITY, 0)", big_integer())
            .contains(r#"CAST(nullif("QUANTITY", 0) AS BIGINT)"#)
    );
    assert!(
        model_sql("ZEROIFNULL(AMOUNT)", decimal(10, 2))
            .contains(r#"CAST(coalesce("AMOUNT", 0) AS DECIMAL(10,2))"#)
    );
    assert!(
        model_sql("NULLIFZERO(AMOUNT)", decimal(10, 2))
            .contains(r#"CAST(nullif("AMOUNT", 0) AS DECIMAL(10,2))"#)
    );
}

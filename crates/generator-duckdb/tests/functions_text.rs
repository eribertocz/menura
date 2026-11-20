//! Unit tests of the text functions and of `LIKE` and `ILIKE` (E0F-10.2).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::LogicalType;
use menura_generator_duckdb::GeneratedSql;
use serde_json::json;
use support::{big_integer, expression_sql, from_model, from_query, query_with_reference_time};

/// The SQL of `expression`, a model expression of type `logical_type`.
fn model_sql(expression: &str, logical_type: LogicalType) -> GeneratedSql {
    expression_sql(
        expression,
        logical_type,
        from_model("/semantic_model/0/datasets/0/fields/0"),
        &query_with_reference_time(),
    )
    .expect("SQL")
}

fn text_sql(expression: &str) -> String {
    model_sql(expression, LogicalType::Text).sql
}

#[test]
fn e0f_10_2_concat_is_null_when_an_argument_is_null() {
    assert!(
        text_sql("CONCAT(REGION, '-', REGION)")
            .contains(r#"CAST("REGION" || '-' || "REGION" AS VARCHAR) AS "X""#)
    );
    assert!(text_sql("CONCAT(REGION)").contains(r#"CAST("REGION" AS VARCHAR)"#));
}

#[test]
fn e0f_10_2_length_counts_characters() {
    let generated = model_sql("LENGTH(REGION)", big_integer());
    assert!(
        generated
            .sql
            .contains(r#"CAST(length("REGION") AS BIGINT)"#)
    );
}

#[test]
fn e0f_10_2_case_mapping() {
    assert!(text_sql("LOWER(REGION)").contains(r#"CAST(lower("REGION") AS VARCHAR)"#));
    assert!(text_sql("UPPER(REGION)").contains(r#"CAST(upper("REGION") AS VARCHAR)"#));
}

#[test]
fn e0f_10_2_trim_removes_spaces_only() {
    assert!(text_sql("TRIM(REGION)").contains(r#"trim("REGION", ' ')"#));
    assert!(text_sql("LTRIM(REGION)").contains(r#"ltrim("REGION", ' ')"#));
    assert!(text_sql("RTRIM(REGION)").contains(r#"rtrim("REGION", ' ')"#));
}

#[test]
fn e0f_10_2_left_and_right_of_a_negative_count_are_null() {
    assert!(
        text_sql("LEFT(REGION, QUANTITY)")
            .contains(r#"left("REGION", CASE WHEN "QUANTITY" < 0 THEN NULL ELSE "QUANTITY" END)"#)
    );
    assert!(
        text_sql("RIGHT(REGION, QUANTITY)")
            .contains(r#"right("REGION", CASE WHEN "QUANTITY" < 0 THEN NULL ELSE "QUANTITY" END)"#)
    );
}

#[test]
fn e0f_10_2_substring_keeps_the_positions_that_exist() {
    assert!(
        text_sql("SUBSTRING(REGION, QUANTITY)")
            .contains(r#"substring("REGION", greatest("QUANTITY", 1))"#)
    );
    assert!(text_sql("SUBSTRING(REGION, QUANTITY, 3)").contains(
        r#"substring("REGION", greatest("QUANTITY", 1), CASE WHEN 3 < 0 THEN NULL ELSE CAST(least(greatest(CAST("QUANTITY" AS HUGEINT) + 3 - greatest("QUANTITY", 1), 0), length("REGION")) AS BIGINT) END)"#
    ));
}

#[test]
fn e0f_10_2_replace_of_empty_text_leaves_the_text_unchanged() {
    assert!(text_sql("REPLACE(REGION, '', 'x')").contains(
        r#"CASE WHEN "REGION" IS NULL OR '' IS NULL OR 'x' IS NULL THEN NULL WHEN '' = '' THEN "REGION" ELSE replace("REGION", '', 'x') END"#
    ));
}

#[test]
fn e0f_10_2_split_part() {
    assert!(text_sql("SPLIT_PART(REGION, '-', 2)").contains(r#"split_part("REGION", '-', 2)"#));
}

#[test]
fn e0f_10_2_position_of_empty_text_is_one() {
    for expression in ["POSITION('o' IN REGION)", "CHARINDEX('o', REGION)"] {
        let generated = model_sql(expression, big_integer());
        assert!(generated.sql.contains(
            r#"CAST(CASE WHEN 'o' = '' AND "REGION" IS NOT NULL THEN 1 ELSE strpos("REGION", 'o') END AS BIGINT)"#
        ));
    }
}

#[test]
fn e0f_10_2_contains_starts_and_ends() {
    let boolean = |expression| model_sql(expression, LogicalType::Boolean).sql;
    assert!(boolean("CONTAINS(REGION, 'or')").contains(r#"contains("REGION", 'or')"#));
    assert!(boolean("STARTSWITH(REGION, 'no')").contains(r#"prefix("REGION", 'no')"#));
    assert!(boolean("ENDSWITH(REGION, 'th')").contains(r#"suffix("REGION", 'th')"#));
}

#[test]
fn e0f_10_2_like_has_no_escape_character() {
    let boolean = |expression| model_sql(expression, LogicalType::Boolean).sql;
    assert!(boolean("REGION LIKE 'n_r%'").contains(r#""REGION" LIKE 'n_r%' AS "X""#));
    assert!(boolean("REGION NOT ILIKE 'N%'").contains(r#""REGION" NOT ILIKE 'N%' AS "X""#));
    assert!(boolean("NOT REGION LIKE 'n%'").contains(r#"NOT ("REGION" LIKE 'n%')"#));
}

#[test]
fn e6_4_2_query_arguments_are_bound_parameters() {
    let generated = expression_sql(
        "LEFT(REGION, 2)",
        LogicalType::Text,
        from_query("/query/filters/0"),
        &query_with_reference_time(),
    )
    .expect("SQL");
    assert!(
        generated
            .sql
            .contains(r#"left("REGION", CASE WHEN $1 < 0 THEN NULL ELSE $1 END)"#)
    );
    assert_eq!(generated.parameters.len(), 1);
    assert_eq!(generated.parameters[0].value, json!(2));
}

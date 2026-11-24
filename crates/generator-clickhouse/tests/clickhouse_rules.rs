//! Unit tests of the `ClickHouse` rules (E6-7) on small plans, each with its exact SQL text.

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::{FloatBits, IntegerBits, LogicalType, TimestampPrecision};
use menura_representations::logical_plan::PlanNode;
use serde_json::json;
use support::{
    SETTINGS, TWO_METRICS, aggregate, assert_golden, big_integer, column, component, decimal,
    filter, from_query, generate, generated, instant, named, project, query,
    query_with_reference_time, region_count, scan, sort, two_branches,
};

/// The fields of `ORDERS` the expressions read: every one but `SHIPPED_AT`, a `timestamp`
/// without time zone, which `ClickHouse` cannot express (E6-7.12).
const FIELDS: &[&str] = &["AMOUNT", "CREATED_AT", "ORDERED_ON", "QUANTITY", "REGION"];

/// The `Project`, from the query, of `ORDERS` whose only column `X` is `expression`.
fn projected(expression: &str, logical_type: LogicalType) -> PlanNode {
    project(
        scan("ORDERS", FIELDS),
        vec![named("X", expression)],
        vec![column("X", logical_type, true)],
        from_query("/query/dimensions/0"),
    )
}

#[cfg(test)]
/// The select item of `X` and the parameters of the statement of `expression`, for the query
/// at the reference time of `query_with_reference_time`, in `America/New_York`.
fn item(expression: &str, logical_type: LogicalType) -> (String, serde_json::Value) {
    let generated = generate(
        &projected(expression, logical_type),
        &query_with_reference_time(),
    )
    .expect("SQL");
    let line = generated
        .sql
        .lines()
        .find(|line| line.ends_with(" AS \"X\""))
        .expect("the item of X")
        .trim()
        .to_owned();
    (
        line,
        serde_json::to_value(&generated.parameters).expect("JSON"),
    )
}

/// Logical types, each with a nullability and the `ClickHouse` name of E6-7.2.
fn named_types() -> Vec<(LogicalType, bool, &'static str)> {
    vec![
        (LogicalType::Boolean, false, "Bool"),
        (
            LogicalType::Integer {
                bits: IntegerBits::Sixteen,
                signed: false,
            },
            false,
            "UInt16",
        ),
        (decimal(12, 2), true, "Nullable(Decimal(12, 2))"),
        (
            LogicalType::Float {
                bits: FloatBits::ThirtyTwo,
            },
            false,
            "Float32",
        ),
        (LogicalType::Text, true, "Nullable(String)"),
        (LogicalType::Binary, false, "String"),
        (LogicalType::Uuid, true, "Nullable(UUID)"),
        (LogicalType::Date, false, "Date32"),
        (
            LogicalType::Timestamp {
                precision: TimestampPrecision::Ns,
                with_time_zone: true,
            },
            true,
            "Nullable(DateTime64(9, 'UTC'))",
        ),
        (
            LogicalType::List {
                element: Box::new(LogicalType::Text),
            },
            false,
            "Array(String)",
        ),
    ]
}

fn boolean() -> LogicalType {
    LogicalType::Boolean
}

#[test]
fn e6_7_1_identifiers_are_double_quoted_and_a_source_names_database_and_table() {
    let plan = project(
        scan("ORDERS", &["REGION"]),
        vec![named("Sales \"Region\"", "REGION")],
        vec![column("Sales \"Region\"", LogicalType::Text, true)],
        from_query("/query/dimensions/0"),
    );
    let generated = generate(&plan, &query(&["Sales \"Region\""], &[])).expect("SQL");
    let expected = format!(
        r#"WITH
  "menura_1_scan" AS (
    SELECT
      "Sales Region" AS "REGION"
    FROM "main"."orders"
  ),
  "menura_2_project" AS (
    SELECT
      "REGION" AS "Sales ""Region"""
    FROM "menura_1_scan"
  )
SELECT
  "Sales ""Region"""
FROM "menura_2_project"
{SETTINGS}"#
    );
    assert_eq!(generated.sql, expected);
}

#[test]
fn e6_7_2_the_result_schema_holds_clickhouse_type_names() {
    let types = named_types();
    let names: Vec<String> = (0..types.len()).map(|index| format!("C{index}")).collect();
    let plan = project(
        scan("ORDERS", &["QUANTITY"]),
        names.iter().map(|name| named(name, "QUANTITY")).collect(),
        names
            .iter()
            .zip(&types)
            .map(|(name, (logical_type, nullable, _))| {
                column(name, logical_type.clone(), *nullable)
            })
            .collect(),
        from_query("/query"),
    );
    let columns: Vec<&str> = names.iter().map(String::as_str).collect();
    let generated = generate(&plan, &query(&columns, &[])).expect("SQL");
    let physical: Vec<&str> = generated
        .result_schema
        .iter()
        .map(|column| column.physical_type.as_str())
        .collect();
    let expected: Vec<&str> = types.iter().map(|(_, _, name)| *name).collect();
    assert_eq!(physical, expected);
}

#[test]
fn e6_7_3_each_parameter_is_written_with_its_type() {
    let plan = filter(
        region_count(),
        "REGION = 'north' AND TOTAL_QUANTITY > 10",
        from_query("/query/filters/0"),
    );
    let generated = generate(
        &plan,
        &query(&["REGION"], &["ORDER_COUNT", "TOTAL_QUANTITY"]),
    )
    .expect("SQL");
    assert!(
        generated.sql.contains(
            "    WHERE (\"REGION\" = {p1:String}) AND (\"TOTAL_QUANTITY\" > {p2:Int64})\n"
        ),
        "{}",
        generated.sql
    );
    assert_eq!(
        serde_json::to_value(&generated.parameters).expect("JSON"),
        json!([
            {"position": 1, "logical_type": {"kind": "text"}, "value": "north"},
            {
                "position": 2,
                "logical_type": {"kind": "integer", "bits": 64, "signed": true},
                "value": 10
            }
        ])
    );
}

#[test]
fn e6_7_3_a_repeated_parameter_repeats_its_typed_placeholder() {
    let (sql, parameters) = item("CREATED_AT >= DATE '2026-01-01'", boolean());
    assert_eq!(
        sql,
        "CAST(\"CREATED_AT\" >= toTimeZone(toDateTime64({p1:Date32}, 6, {p2:String}), 'UTC') \
         AS Nullable(Bool)) AS \"X\""
    );
    assert_eq!(
        parameters,
        json!([
            {"position": 1, "logical_type": {"kind": "date"}, "value": "2026-01-01"},
            {"position": 2, "logical_type": {"kind": "text"}, "value": "America/New_York"}
        ])
    );
}

#[test]
fn e6_7_4_is_not_distinct_from_is_a_null_safe_equality() {
    let (sql, _) = item("REGION IS NOT DISTINCT FROM 'north'", boolean());
    assert_eq!(
        sql,
        "CAST(coalesce(\"REGION\" = {p1:String}, \"REGION\" IS NULL AND {p1:String} IS NULL) \
         AS Nullable(Bool)) AS \"X\""
    );
    let (sql, _) = item("REGION IS DISTINCT FROM 'north'", boolean());
    assert_eq!(
        sql,
        "CAST(NOT coalesce(\"REGION\" = {p1:String}, \"REGION\" IS NULL AND {p1:String} IS NULL) \
         AS Nullable(Bool)) AS \"X\""
    );
}

#[test]
fn e6_7_4_stitched_keys_are_joined_with_a_null_safe_equality() {
    let sql = generated(&two_branches(), &TWO_METRICS).sql;
    assert_golden(
        &sql,
        include_str!("golden/e6_7_4_stitch_null_safe_keys.sql").trim_end(),
    );
}

#[test]
fn e6_7_9_a_division_casts_both_operands_explicitly() {
    let (sql, _) = item(
        "AMOUNT / QUANTITY",
        LogicalType::Float {
            bits: FloatBits::SixtyFour,
        },
    );
    assert_eq!(
        sql,
        "CAST(CASE WHEN toFloat64(\"QUANTITY\") = 0 THEN NULL \
         ELSE toFloat64(\"AMOUNT\") / toFloat64(\"QUANTITY\") END AS Nullable(Float64)) AS \"X\""
    );
}

#[test]
fn e6_7_6_a_timestamp_is_truncated_in_the_query_time_zone() {
    let (sql, parameters) = item("DATE_TRUNC('month', CREATED_AT)", instant());
    assert_eq!(
        sql,
        "CAST(toTimeZone(toDateTime64(toStartOfMonth(\"CREATED_AT\", {p1:String}), \
         6, {p1:String}), 'UTC') AS Nullable(DateTime64(6, 'UTC'))) AS \"X\""
    );
    assert_eq!(
        parameters,
        json!([{"position": 1, "logical_type": {"kind": "text"}, "value": "America/New_York"}])
    );
    let (sql, _) = item("DATE_TRUNC('hour', CREATED_AT)", instant());
    assert_eq!(
        sql,
        "CAST(toTimeZone(toStartOfHour(\"CREATED_AT\", {p1:String}), 'UTC') \
         AS Nullable(DateTime64(6, 'UTC'))) AS \"X\""
    );
}

#[test]
fn e6_7_6_iso_weeks_start_on_monday_and_a_date_stays_a_date() {
    let (sql, parameters) = item("DATE_TRUNC('week', ORDERED_ON)", LogicalType::Date);
    assert_eq!(
        sql,
        "CAST(toDate32(toMonday(\"ORDERED_ON\")) AS Nullable(Date32)) AS \"X\""
    );
    assert_eq!(parameters, json!([]));
}

#[test]
fn e6_7_7_a_list_keeps_standard_null_semantics() {
    let (sql, _) = item("REGION IN ('north', 'south')", boolean());
    assert_eq!(
        sql,
        "CAST(\"REGION\" = {p1:String} OR \"REGION\" = {p2:String} AS Nullable(Bool)) AS \"X\""
    );
    let (sql, _) = item("REGION NOT IN ('north', NULL)", boolean());
    assert_eq!(
        sql,
        "CAST(NOT (\"REGION\" = {p1:String} OR \"REGION\" = NULL) AS Nullable(Bool)) AS \"X\""
    );
}

#[test]
fn e6_7_7_membership_in_an_empty_list_attribute_is_false() {
    let (sql, _) = item("REGION IN (USER_ATTRIBUTE('regions'))", boolean());
    assert!(
        sql.starts_with(
            "CAST(CASE WHEN empty({p1:Array(String)}) THEN FALSE WHEN \"REGION\" IS NULL"
        ),
        "{sql}"
    );
}

#[test]
fn e6_7_7_a_list_attribute_is_an_array_parameter() {
    let (sql, parameters) = item("REGION NOT IN (USER_ATTRIBUTE('regions'))", boolean());
    assert_eq!(
        sql,
        "CAST(NOT (CASE WHEN empty({p1:Array(String)}) THEN FALSE \
         WHEN \"REGION\" IS NULL THEN NULL \
         WHEN has({p1:Array(String)}, assumeNotNull(\"REGION\")) THEN TRUE \
         WHEN has({p1:Array(String)}, NULL) THEN NULL ELSE FALSE END) AS Nullable(Bool)) AS \"X\""
    );
    assert_eq!(
        parameters,
        json!([{
            "position": 1,
            "logical_type": {"kind": "list", "element": {"kind": "text"}},
            "value": ["north", "south"]
        }])
    );
}

#[test]
fn e6_7_8_every_statement_ends_with_its_settings() {
    let generated = generate(
        &region_count(),
        &query(&["REGION"], &["ORDER_COUNT", "TOTAL_QUANTITY"]),
    )
    .expect("SQL");
    assert!(
        generated
            .sql
            .ends_with(&format!("\nFROM \"menura_2_aggregate\"\n{SETTINGS}"))
    );
}

#[test]
fn e6_7_9_a_narrowing_cast_rounds_half_away_from_zero() {
    let integer = LogicalType::Integer {
        bits: IntegerBits::ThirtyTwo,
        signed: true,
    };
    let (sql, _) = item("CAST(AMOUNT AS INTEGER)", integer);
    assert_eq!(
        sql,
        "accurateCast(accurateCast(round(\"AMOUNT\", 0), 'Nullable(Int32)'), 'Nullable(Int32)') AS \"X\""
    );
    let (sql, _) = item("CAST(QUANTITY AS DECIMAL(12,2))", decimal(12, 2));
    assert_eq!(
        sql,
        "CAST(CAST(\"QUANTITY\" AS Nullable(Decimal(12, 2))) AS Nullable(Decimal(12, 2))) AS \"X\""
    );
}

#[test]
fn e6_7_9_a_cast_from_text_rounds_half_away_from_zero() {
    let (sql, _) = item("CAST(REGION AS DECIMAL(12,2))", decimal(12, 2));
    assert_eq!(
        sql,
        "CAST(CAST(round(CAST(\"REGION\" AS Nullable(Decimal(76, 3))), 2) AS Nullable(Decimal(12, 2))) \
         AS Nullable(Decimal(12, 2))) AS \"X\""
    );
    let integer = LogicalType::Integer {
        bits: IntegerBits::ThirtyTwo,
        signed: true,
    };
    let (sql, _) = item("CAST(REGION AS INTEGER)", integer);
    assert_eq!(
        sql,
        "accurateCast(accurateCast(round(CAST(\"REGION\" AS Nullable(Decimal(76, 1))), 0), \
         'Nullable(Int32)'), 'Nullable(Int32)') AS \"X\""
    );
}

#[test]
fn e6_7_9_integer_arithmetic_is_checked_instead_of_wrapping() {
    let (sql, _) = item("QUANTITY * QUANTITY", big_integer());
    assert_eq!(
        sql,
        "accurateCast(accurateCast(toInt256(\"QUANTITY\") * toInt256(\"QUANTITY\"), \
         'Nullable(Int128)'), 'Nullable(Int64)') AS \"X\""
    );
}

#[test]
fn e6_7_14_float_comparisons_treat_nan_as_duckdb_does() {
    let (sql, _) = item("CAST(AMOUNT AS DOUBLE) > 1.5", boolean());
    assert_eq!(
        sql,
        "CAST(CASE WHEN CAST(\"AMOUNT\" AS Nullable(Float64)) IS NULL OR {p1:Decimal(2, 1)} IS NULL \
         THEN NULL ELSE (isNaN(CAST(\"AMOUNT\" AS Nullable(Float64))), \
         if(isNaN(CAST(\"AMOUNT\" AS Nullable(Float64))), 0, CAST(\"AMOUNT\" AS Nullable(Float64)))) \
         > (isNaN({p1:Decimal(2, 1)}), if(isNaN({p1:Decimal(2, 1)}), 0, {p1:Decimal(2, 1)})) END \
         AS Nullable(Bool)) AS \"X\""
    );
}

#[test]
fn e6_7_15_every_order_key_writes_its_null_order() {
    let ratio = LogicalType::Float {
        bits: FloatBits::SixtyFour,
    };
    let plan = sort(
        projected("CAST(AMOUNT AS DOUBLE)", ratio),
        &[("X", true, false)],
    );
    let generated = generate(&plan, &query_with_reference_time()).expect("SQL");
    assert!(
        generated.sql.contains(
            "ORDER BY \"X\" IS NULL ASC NULLS LAST, isNaN(\"X\") DESC NULLS LAST, \"X\" DESC NULLS LAST"
        ),
        "{}",
        generated.sql
    );
}

#[test]
fn e6_7_9_an_integer_average_sums_without_wrapping() {
    let plan = aggregate(
        scan("ORDERS", &["QUANTITY", "REGION"]),
        vec![named("REGION", "REGION")],
        vec![component("AVERAGE_QUANTITY", "AVG(QUANTITY)", None)],
        vec![
            column("REGION", LogicalType::Text, true),
            column(
                "AVERAGE_QUANTITY",
                LogicalType::Float {
                    bits: FloatBits::SixtyFour,
                },
                true,
            ),
        ],
    );
    let generated = generate(&plan, &query(&["REGION"], &["AVERAGE_QUANTITY"])).expect("SQL");
    assert!(
        generated.sql.contains(
            "CAST(CASE WHEN coalesce(count(\"QUANTITY\"), 0) = 0 THEN NULL \
             ELSE toFloat64(sum(toInt256(\"QUANTITY\"))) / toFloat64(count(\"QUANTITY\")) END \
             AS Nullable(Float64)) AS \"AVERAGE_QUANTITY\""
        ),
        "{}",
        generated.sql
    );
}

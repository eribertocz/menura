//! Unit tests of the `DuckDB` rules (E6-5), of the fixed result types of aggregates (E6-4.5) and
//! of the encoding of query values (E6-4.2).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::{FloatBits, IntegerBits, LogicalType, TimestampPrecision};
use menura_generator_duckdb::InvariantViolation;
use menura_model::expression::AggregateCall;
use menura_representations::logical_plan::PlanNode;
use serde_json::json;
use support::{
    aggregate, big_integer, column, component, component_of, decimal, filter, from_model,
    from_query, generate, generate_sql, instant, model, named, project, query, region_count, scan,
    security_with,
};

#[test]
fn e6_5_1_identifiers_are_double_quoted() {
    let plan = project(
        scan("ORDERS", &["REGION"]),
        vec![named("Sales \"Region\"", "REGION")],
        vec![column("Sales \"Region\"", LogicalType::Text, true)],
        from_query("/query"),
    );
    let generated = generate(&plan, &query(&["Sales \"Region\""], &[])).expect("SQL");
    assert!(generated.sql.contains(r#""REGION" AS "Sales ""Region""""#));
}

#[test]
fn e6_5_4_division_and_truncation_never_rely_on_the_engine() {
    let plan = project(
        region_count(),
        vec![
            named("REGION", "REGION"),
            named("PER_ORDER", "TOTAL_QUANTITY / ORDER_COUNT"),
        ],
        vec![
            column("REGION", LogicalType::Text, true),
            column(
                "PER_ORDER",
                LogicalType::Float {
                    bits: FloatBits::SixtyFour,
                },
                true,
            ),
        ],
        from_model("/semantic_model/0/metrics/0"),
    );
    let generated = generate(&plan, &query(&["REGION"], &["PER_ORDER"])).expect("SQL");
    let divisions = generated.sql.matches(" / ").count();
    assert_eq!(divisions, 1);
    assert!(generated.sql.contains("AS DOUBLE) / CAST("));
}

/// A `Project` that declares `QUANTITY` under each of `types`.
fn typed_columns(types: &[LogicalType]) -> PlanNode {
    let names: Vec<String> = (0..types.len()).map(|index| format!("C{index}")).collect();
    project(
        scan("ORDERS", &["QUANTITY"]),
        names.iter().map(|name| named(name, "QUANTITY")).collect(),
        names
            .iter()
            .zip(types)
            .map(|(name, logical_type)| column(name, logical_type.clone(), true))
            .collect(),
        from_query("/query"),
    )
}

/// Every logical type of CAT-4.4 with its `DuckDB` name.
fn physical_names() -> Vec<(LogicalType, &'static str)> {
    let integer = |bits, signed| LogicalType::Integer { bits, signed };
    let timestamp = |precision| LogicalType::Timestamp {
        precision,
        with_time_zone: false,
    };
    let float = |bits| LogicalType::Float { bits };
    let list = |element| LogicalType::List {
        element: Box::new(element),
    };
    [
        (LogicalType::Boolean, "BOOLEAN"),
        (integer(IntegerBits::Eight, true), "TINYINT"),
        (integer(IntegerBits::Sixteen, true), "SMALLINT"),
        (integer(IntegerBits::ThirtyTwo, true), "INTEGER"),
        (integer(IntegerBits::SixtyFour, true), "BIGINT"),
        (integer(IntegerBits::OneHundredTwentyEight, true), "HUGEINT"),
        (integer(IntegerBits::Eight, false), "UTINYINT"),
        (integer(IntegerBits::Sixteen, false), "USMALLINT"),
        (integer(IntegerBits::ThirtyTwo, false), "UINTEGER"),
        (integer(IntegerBits::SixtyFour, false), "UBIGINT"),
        (
            integer(IntegerBits::OneHundredTwentyEight, false),
            "UHUGEINT",
        ),
        (decimal(18, 2), "DECIMAL(18,2)"),
        (float(FloatBits::ThirtyTwo), "FLOAT"),
        (float(FloatBits::SixtyFour), "DOUBLE"),
        (LogicalType::Text, "VARCHAR"),
        (LogicalType::Binary, "BLOB"),
        (LogicalType::Uuid, "UUID"),
        (LogicalType::Date, "DATE"),
        (LogicalType::Time, "TIME"),
        (LogicalType::Interval, "INTERVAL"),
        (timestamp(TimestampPrecision::S), "TIMESTAMP_S"),
        (timestamp(TimestampPrecision::Ms), "TIMESTAMP_MS"),
        (timestamp(TimestampPrecision::Us), "TIMESTAMP"),
        (timestamp(TimestampPrecision::Ns), "TIMESTAMP_NS"),
        (instant(), "TIMESTAMP WITH TIME ZONE"),
        (list(LogicalType::Text), "VARCHAR[]"),
    ]
    .to_vec()
}

#[test]
fn e6_5_6_physical_types_are_duckdb_names() {
    let cases = physical_names();
    let types: Vec<LogicalType> = cases.iter().map(|(logical, _)| logical.clone()).collect();
    let names: Vec<String> = (0..types.len()).map(|index| format!("C{index}")).collect();
    let dimensions: Vec<&str> = names.iter().map(String::as_str).collect();
    let generated = generate(&typed_columns(&types), &query(&dimensions, &[])).expect("SQL");
    let physical: Vec<&str> = generated
        .result_schema
        .iter()
        .map(|column| column.physical_type.as_str())
        .collect();
    let expected: Vec<&str> = cases.iter().map(|(_, name)| *name).collect();
    assert_eq!(physical, expected);
}

#[test]
fn e6_5_6_unsupported_types_have_no_physical_name() {
    let plan = typed_columns(&[LogicalType::Unsupported]);
    let error = generate(&plan, &query(&["C0"], &[])).unwrap_err();
    assert_eq!(error.violation, InvariantViolation::UnsupportedType);
}

#[test]
fn e6_5_7_attribute_parameters_have_their_declared_types() {
    let plan = filter(
        scan("ORDERS", &["AMOUNT", "QUANTITY", "REGION"]),
        "AMOUNT <= USER_ATTRIBUTE('max_amount') AND QUANTITY = USER_ATTRIBUTE('tenant') \
         AND REGION IN (USER_ATTRIBUTE('regions')) AND QUANTITY IN (USER_ATTRIBUTE('tenants'))",
        from_model("/semantic_model/0/datasets/0/row_policies/0"),
    );
    let generated = generate(&plan, &query(&["AMOUNT", "QUANTITY", "REGION"], &[])).expect("SQL");
    let parameters: Vec<_> = generated
        .parameters
        .iter()
        .map(|parameter| (parameter.logical_type.clone(), parameter.value.clone()))
        .collect();
    let list = |element| LogicalType::List {
        element: Box::new(element),
    };
    assert_eq!(
        parameters,
        vec![
            (decimal(10, 2), json!("100.50")),
            (big_integer(), json!(7)),
            (list(LogicalType::Text), json!(["north", "south"])),
            (list(big_integer()), json!([1, 2])),
        ]
    );
}

#[test]
fn e6_5_7_a_missing_attribute_is_an_internal_error() {
    let plan = filter(
        scan("ORDERS", &["QUANTITY"]),
        "QUANTITY = USER_ATTRIBUTE('tenant')",
        from_model("/semantic_model/0/datasets/0/row_policies/0"),
    );
    let generated = generate_sql(
        &plan,
        &model(),
        &query(&["QUANTITY"], &[]),
        &security_with("{}"),
    );
    assert_eq!(
        generated.unwrap_err().violation,
        InvariantViolation::MissingAttribute
    );
}

#[test]
fn e6_4_2_query_values_are_encoded_as_con_4_requires() {
    let plan = filter(
        scan(
            "ORDERS",
            &["AMOUNT", "CREATED_AT", "ORDERED_ON", "QUANTITY"],
        ),
        "ORDERED_ON >= DATE '2026-01-01' AND AMOUNT < 007.50 AND QUANTITY < 12345678901234567890 \
         AND CAST(ORDERED_ON AS TIMESTAMP) < TIMESTAMP '2026-01-01 10:00:00.500' AND QUANTITY < 2.5e1",
        from_query("/query/filters/0"),
    );
    let generated = generate(
        &plan,
        &query(&["AMOUNT", "CREATED_AT", "ORDERED_ON", "QUANTITY"], &[]),
    )
    .expect("SQL");
    let parameters: Vec<_> = generated
        .parameters
        .iter()
        .map(|parameter| (parameter.logical_type.clone(), parameter.value.clone()))
        .collect();
    assert_eq!(
        parameters,
        vec![
            (LogicalType::Date, json!("2026-01-01")),
            (decimal(5, 2), json!("7.50")),
            (decimal(20, 0), json!("12345678901234567890")),
            (
                LogicalType::Timestamp {
                    precision: TimestampPrecision::Us,
                    with_time_zone: false,
                },
                json!("2026-01-01T10:00:00.5"),
            ),
            (
                LogicalType::Float {
                    bits: FloatBits::SixtyFour,
                },
                json!(25.0),
            ),
        ]
    );
}

#[test]
fn e6_4_5_every_aggregate_has_its_fixed_type() {
    let float = LogicalType::Float {
        bits: FloatBits::SixtyFour,
    };
    let plan = aggregate(
        scan("ORDERS", &["AMOUNT", "QUANTITY"]),
        Vec::new(),
        vec![
            component("DISTINCT_QUANTITIES", "COUNT(DISTINCT QUANTITY)", None),
            component("SPREAD", "STDDEV(AMOUNT)", None),
            component(
                "P90",
                "PERCENTILE_CONT(0.9) WITHIN GROUP (ORDER BY AMOUNT DESC)",
                None,
            ),
            component("LARGEST", "MAX(AMOUNT)", None),
            component_of("ROWS", AggregateCall::CountRows, Some("AMOUNT > 0")),
        ],
        vec![
            column("DISTINCT_QUANTITIES", big_integer(), false),
            column("SPREAD", float.clone(), true),
            column("P90", float, true),
            column("LARGEST", decimal(10, 2), true),
            column("ROWS", big_integer(), false),
        ],
    );
    let columns = query(
        &[],
        &["DISTINCT_QUANTITIES", "SPREAD", "P90", "LARGEST", "ROWS"],
    );
    let generated = generate(&plan, &columns).expect("SQL");
    for expected in [
        r#"CAST(COUNT(DISTINCT "QUANTITY") AS BIGINT) AS "DISTINCT_QUANTITIES""#,
        r#"CAST(STDDEV_SAMP("AMOUNT") AS DOUBLE) AS "SPREAD""#,
        r#"CAST(PERCENTILE_CONT(0.9) WITHIN GROUP (ORDER BY CAST("AMOUNT" AS DOUBLE) DESC) AS DOUBLE) AS "P90""#,
        r#"CAST(MAX("AMOUNT") AS DECIMAL(10,2)) AS "LARGEST""#,
        r#"CAST(COUNT(*) FILTER (WHERE "AMOUNT" > $1) AS BIGINT) AS "ROWS""#,
    ] {
        assert!(
            generated.sql.contains(expected),
            "{expected} in {}",
            generated.sql
        );
    }
}

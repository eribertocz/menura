//! Unit tests of the rules of E6-1 to E6-4 whose subject is not only the SQL text.

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::{FloatBits, LogicalType};
use menura_contracts::types::compiled_plan::ColumnRole;
use menura_generator_duckdb::InvariantViolation;
use menura_representations::logical_plan::{Operation, OutputSchema, PlanNode};
use menura_representations::names::ColumnName;
use serde_json::json;
use support::{
    aggregate, big_integer, column, component, filter, from_model, from_query, generate,
    generate_sql, instant, limit, model, named, project, query, query_in, region_count, scan,
    security,
};

#[test]
fn e6_1_the_plan_becomes_sql_parameters_and_result_schema() {
    let plan = filter(
        region_count(),
        "ORDER_COUNT > 3",
        from_query("/query/filters/0"),
    );
    let generated = generate(
        &plan,
        &query(&["REGION"], &["ORDER_COUNT", "TOTAL_QUANTITY"]),
    )
    .expect("SQL");
    assert!(generated.sql.starts_with("WITH\n"));
    assert_eq!(generated.parameters.len(), 1);
    assert_eq!(generated.result_schema.len(), 3);
}

#[test]
fn e6_2_result_schema_follows_the_output_columns() {
    let generated = generate(
        &region_count(),
        &query(&["REGION"], &["ORDER_COUNT", "TOTAL_QUANTITY"]),
    )
    .expect("SQL");
    let schema: Vec<_> = generated
        .result_schema
        .iter()
        .map(|column| {
            (
                column.name.as_str(),
                column.role,
                column.physical_type.as_str(),
                column.nullable,
            )
        })
        .collect();
    assert_eq!(
        schema,
        vec![
            ("REGION", ColumnRole::Dimension, "VARCHAR", true),
            ("ORDER_COUNT", ColumnRole::Metric, "BIGINT", false),
            ("TOTAL_QUANTITY", ColumnRole::Metric, "HUGEINT", true),
        ]
    );
    assert_eq!(generated.result_schema[1].logical_type, big_integer());
}

#[test]
fn e6_2_a_result_schema_that_does_not_match_the_query_is_an_error() {
    let error = generate(&region_count(), &query(&["REGION"], &[])).unwrap_err();
    assert_eq!(error.violation, InvariantViolation::ResultColumnMismatch);
}

fn node(operation: Operation) -> PlanNode {
    PlanNode::new(
        operation,
        OutputSchema::new(vec![column("REGION", LogicalType::Text, true)]),
        from_query("/query"),
    )
}

#[test]
fn e6_3_1_nodes_outside_the_slice_are_internal_errors() {
    let input = || Box::new(scan("ORDERS", &["REGION"]));
    let nodes = [
        node(Operation::SemiJoin {
            input: input(),
            path: Vec::new(),
            predicate: support::parse("TRUE"),
        }),
        node(Operation::Stitch {
            branches: vec![scan("ORDERS", &["REGION"])],
            keys: vec![ColumnName::new("REGION")],
            columns: Vec::new(),
        }),
    ];
    for plan in nodes {
        let error = generate(&plan, &query(&["REGION"], &[])).unwrap_err();
        assert_eq!(error.violation, InvariantViolation::NodeOutsideSlice);
        assert_eq!(error.origin, from_query("/query"));
    }
}

#[test]
fn e6_3_1_inconsistent_plans_are_internal_errors() {
    let cases = [
        (
            scan("CUSTOMERS", &["REGION"]),
            InvariantViolation::UnknownDataset,
        ),
        (
            filter(
                scan("ORDERS", &["REGION"]),
                "MISSING = 1",
                from_query("/query"),
            ),
            InvariantViolation::UnknownColumn,
        ),
        (
            filter(
                scan("ORDERS", &["REGION"]),
                "ORDERS.REGION = 'a'",
                from_query("/query"),
            ),
            InvariantViolation::QualifiedReference,
        ),
        (
            filter(
                scan("ORDERS", &["REGION"]),
                "AGG(REGION) = 'A'",
                from_query("/query"),
            ),
            InvariantViolation::ExpressionOutsideSlice,
        ),
        (
            filter(
                scan("ORDERS", &["REGION"]),
                "REGION = USER_ATTRIBUTE('regions')",
                from_model("/semantic_model/0/datasets/0/row_policies/0"),
            ),
            InvariantViolation::MisplacedListAttribute,
        ),
        (
            filter(
                scan("ORDERS", &["REGION"]),
                "REGION = USER_ATTRIBUTE('unknown')",
                from_model("/semantic_model/0/datasets/0/row_policies/0"),
            ),
            InvariantViolation::UndeclaredAttribute,
        ),
    ];
    for (plan, violation) in cases {
        let error = generate(&plan, &query(&["REGION"], &[])).unwrap_err();
        assert_eq!(error.violation, violation);
    }
}

#[test]
fn e6_3_2_dialect_knowledge_stays_in_the_generator() {
    let plan = aggregate(
        scan("ORDERS", &["CREATED_AT"]),
        vec![named("CREATED_AT_MONTH", "DATE_TRUNC('month', CREATED_AT)")],
        Vec::new(),
        vec![column("CREATED_AT_MONTH", instant(), true)],
    );
    let generated = generate(&plan, &query(&["CREATED_AT_MONTH"], &[])).expect("SQL");
    assert!(
        generated
            .sql
            .contains(r#"date_trunc('month', "CREATED_AT" AT TIME ZONE CAST($1 AS VARCHAR))"#)
    );
    assert_eq!(
        generated.result_schema[0].physical_type,
        "TIMESTAMP WITH TIME ZONE"
    );
}

#[test]
fn e6_4_1_one_read_only_statement() {
    for plan in [
        scan("ORDERS", &["REGION"]),
        limit(scan("ORDERS", &["REGION"]), 1),
    ] {
        let generated = generate(&plan, &query(&["REGION"], &[])).expect("SQL");
        assert!(generated.sql.starts_with("WITH\n"));
        assert!(!generated.sql.contains(';'));
        assert_eq!(generated.sql.matches("\nSELECT\n").count(), 1);
    }
}

#[test]
fn e6_4_2_parameters_in_order_of_first_appearance() {
    let scanned = scan("ORDERS", &["CREATED_AT", "QUANTITY", "REGION"]);
    let policy = filter(
        scanned,
        "QUANTITY = USER_ATTRIBUTE('tenant') OR REGION IN (USER_ATTRIBUTE('regions'))",
        from_model("/semantic_model/0/datasets/0/row_policies/0"),
    );
    let queried = filter(
        policy,
        "QUANTITY BETWEEN 1 AND 5 AND QUANTITY <> USER_ATTRIBUTE('tenant')",
        from_query("/query/filters/0"),
    );
    let plan = aggregate(
        queried,
        vec![
            named("CREATED_AT_DAY", "DATE_TRUNC('day', CREATED_AT)"),
            named("CREATED_AT_MONTH", "DATE_TRUNC('month', CREATED_AT)"),
        ],
        Vec::new(),
        vec![
            column("CREATED_AT_DAY", instant(), true),
            column("CREATED_AT_MONTH", instant(), true),
        ],
    );
    let generated =
        generate(&plan, &query(&["CREATED_AT_DAY", "CREATED_AT_MONTH"], &[])).expect("SQL");
    let values: Vec<_> = generated
        .parameters
        .iter()
        .map(|parameter| (parameter.position, parameter.value.clone()))
        .collect();
    assert_eq!(
        values,
        vec![
            (1, json!(7)),
            (2, json!(["north", "south"])),
            (3, json!(1)),
            (4, json!(5)),
            (5, json!("Europe/Madrid")),
        ]
    );
    let first_appearances: Vec<usize> = (1..=5)
        .map(|position| generated.sql.find(&format!("${position}")).expect("used"))
        .collect();
    assert!(first_appearances.is_sorted());
    assert!(
        generated
            .sql
            .contains(r#""QUANTITY" <> CAST($1 AS BIGINT)"#)
    );
}

#[test]
fn e6_4_5_metric_results_are_cast_to_their_type() {
    let generated = generate(
        &region_count(),
        &query(&["REGION"], &["ORDER_COUNT", "TOTAL_QUANTITY"]),
    )
    .expect("SQL");
    assert!(
        generated
            .sql
            .contains(r#"CAST(COUNT(*) AS BIGINT) AS "ORDER_COUNT""#)
    );
    assert!(
        generated
            .sql
            .contains(r#"CAST(SUM("QUANTITY") AS HUGEINT) AS "TOTAL_QUANTITY""#)
    );
}

#[test]
fn e6_4_6_divisions_by_zero_give_null() {
    let plan = project(
        region_count(),
        vec![named("RATIO", "TOTAL_QUANTITY / ORDER_COUNT")],
        vec![column("RATIO", support::decimal(38, 0), true)],
        from_model("/semantic_model/0/metrics/0"),
    );
    let generated = generate(&plan, &query(&[], &["RATIO"])).expect("SQL");
    assert!(generated.sql.contains(
        r#"CASE WHEN CAST("ORDER_COUNT" AS DOUBLE) = 0 THEN NULL ELSE CAST("TOTAL_QUANTITY" AS DOUBLE) / CAST("ORDER_COUNT" AS DOUBLE) END"#
    ));
}

#[test]
fn e6_4_10_sql_does_not_depend_on_session_settings() {
    let plan = aggregate(
        scan("ORDERS", &["AMOUNT", "CREATED_AT", "QUANTITY"]),
        vec![named("CREATED_AT_YEAR", "DATE_TRUNC('year', CREATED_AT)")],
        vec![component("AVERAGE_QUANTITY", "AVG(QUANTITY)", None)],
        vec![
            column("CREATED_AT_YEAR", instant(), true),
            column(
                "AVERAGE_QUANTITY",
                LogicalType::Float {
                    bits: FloatBits::SixtyFour,
                },
                true,
            ),
        ],
    );
    let generated =
        generate(&plan, &query(&["CREATED_AT_YEAR"], &["AVERAGE_QUANTITY"])).expect("SQL");
    assert!(generated.sql.contains(
        r#"date_trunc('year', "CREATED_AT" AT TIME ZONE CAST($1 AS VARCHAR)) AT TIME ZONE CAST($1 AS VARCHAR) AS "CREATED_AT_YEAR""#
    ));
    assert!(generated.sql.contains(
        r#"CASE WHEN COUNT("QUANTITY") = 0 THEN NULL ELSE CAST(SUM("QUANTITY") AS DOUBLE) / CAST(COUNT("QUANTITY") AS DOUBLE) END"#
    ));
}

#[test]
fn e6_4_10_the_time_zone_is_the_query_time_zone() {
    let plan = aggregate(
        scan("ORDERS", &["CREATED_AT"]),
        vec![named("CREATED_AT_DAY", "DATE_TRUNC('day', CREATED_AT)")],
        Vec::new(),
        vec![column("CREATED_AT_DAY", instant(), true)],
    );
    let generated = generate_sql(
        &plan,
        &model(),
        &query_in(&["CREATED_AT_DAY"], &[], "America/Lima"),
        &security(),
    )
    .expect("SQL");
    assert_eq!(generated.parameters[0].value, json!("America/Lima"));
}

#[test]
fn e6_4_11_only_closed_or_validated_values_are_literals() {
    let scanned = scan("ORDERS", &["IS_PAID", "REGION"]);
    let denied = filter(
        scanned,
        "FALSE OR REGION = 'north'",
        from_model("/semantic_model/0/datasets/0/row_policies/0"),
    );
    let queried = filter(denied, "REGION = 'south'", from_query("/query/filters/0"));
    let plan = limit(queried, 25);
    let generated = generate(&plan, &query(&["IS_PAID", "REGION"], &[])).expect("SQL");
    assert!(generated.sql.contains(r#""status" = 'paid' AS "IS_PAID""#));
    assert!(
        generated
            .sql
            .contains(r#"WHERE FALSE OR ("REGION" = 'north')"#)
    );
    assert!(
        generated
            .sql
            .contains(r#"WHERE "REGION" = CAST($1 AS VARCHAR)"#)
    );
    assert!(generated.sql.contains("LIMIT 25"));
    assert!(!generated.sql.contains("south"));
    assert_eq!(generated.parameters[0].value, json!("south"));
}

#[test]
fn e6_4_11_model_constants_are_written_exactly() {
    let plan = filter(
        scan("ORDERS", &["AMOUNT", "QUANTITY", "REGION"]),
        "REGION <> 'O''Hara' AND AMOUNT > 1.50 AND QUANTITY < 10 AND REGION IS NOT NULL",
        from_model("/semantic_model/0/datasets/0/row_policies/0"),
    );
    let generated = generate(&plan, &query(&["AMOUNT", "QUANTITY", "REGION"], &[])).expect("SQL");
    assert!(generated.sql.contains(
        r#"(("REGION" <> 'O''Hara') AND ("AMOUNT" > CAST('1.50' AS DECIMAL(3,2)))) AND ("QUANTITY" < 10)"#
    ));
    assert!(generated.parameters.is_empty());
}

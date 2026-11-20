//! Golden SQL tests: the rules whose subject is the SQL text (E6-4.3, E6-4.4, E6-4.8, E6-4.9,
//! E6-5.2, E6-5.3) and the SQL shapes the `core` issue then runs on `DuckDB`.

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::LogicalType;
use serde_json::json;
use support::{
    aggregate, assert_golden, big_integer, column, component, decimal, filter, from_model,
    from_query, generate, generate_sql, limit, model, model_with_source, named, project, query,
    scan, security, security_with, sort,
};

use menura_representations::logical_plan::PlanNode;

fn huge_integer() -> LogicalType {
    LogicalType::Integer {
        bits: menura_contracts::types::common::IntegerBits::OneHundredTwentyEight,
        signed: true,
    }
}

fn double() -> LogicalType {
    LogicalType::Float {
        bits: menura_contracts::types::common::FloatBits::SixtyFour,
    }
}

/// Scan, query filter, aggregate, project, sort and limit: every node of the 0.3.0 slice.
fn full_chain() -> PlanNode {
    let filtered = filter(
        scan("ORDERS", &["AMOUNT", "REGION"]),
        "REGION <> 'west'",
        from_query("/query/filters/0"),
    );
    let counted = aggregate(
        filtered,
        vec![named("REGION", "REGION")],
        vec![
            component("ORDER_COUNT", "COUNT(*)", None),
            component("TOTAL_AMOUNT", "SUM(AMOUNT)", None),
        ],
        vec![
            column("REGION", LogicalType::Text, true),
            column("ORDER_COUNT", big_integer(), false),
            column("TOTAL_AMOUNT", decimal(38, 2), true),
        ],
    );
    let projected = project(
        counted,
        vec![
            named("REGION", "REGION"),
            named("ORDER_COUNT", "ORDER_COUNT"),
            named("TOTAL_AMOUNT", "TOTAL_AMOUNT"),
        ],
        vec![
            column("REGION", LogicalType::Text, true),
            column("ORDER_COUNT", big_integer(), false),
            column("TOTAL_AMOUNT", decimal(38, 2), true),
        ],
        from_query("/query"),
    );
    limit(sort(projected, &[("TOTAL_AMOUNT", true, false)]), 10)
}

#[test]
fn e6_4_3_relation_source_parts_are_quoted() {
    let plan = scan("ORDERS", &["REGION"]);
    let model = model_with_source(r#"warehouse."my.schema"."Order ""Lines""""#);
    let generated =
        generate_sql(&plan, &model, &query(&["REGION"], &[]), &security()).expect("generated SQL");
    assert_golden(
        &generated.sql,
        include_str!("golden/e6_4_3_relation_source.sql"),
    );
}

#[test]
fn e6_4_3_query_source_is_a_subquery() {
    let plan = scan("STAGED", &["REGION"]);
    let generated = generate(&plan, &query(&["REGION"], &[])).expect("generated SQL");
    assert_golden(
        &generated.sql,
        include_str!("golden/e6_4_3_query_source.sql"),
    );
}

#[test]
fn e6_4_3_a_name_that_starts_with_a_keyword_is_a_relation() {
    let plan = scan("ORDERS", &["REGION"]);
    for (source, from) in [
        (
            "withdrawals.main.entries",
            r#"FROM "withdrawals"."main"."entries""#,
        ),
        ("selections", r#"FROM "selections""#),
        ("with$table", r#"FROM "with$table""#),
    ] {
        let model = model_with_source(source);
        let generated = generate_sql(&plan, &model, &query(&["REGION"], &[]), &security())
            .expect("generated SQL");
        assert!(generated.sql.contains(from), "{source}: {}", generated.sql);
    }
}

#[test]
fn e6_4_3_a_keyword_followed_by_a_separator_is_a_query() {
    let plan = scan("ORDERS", &["REGION"]);
    for source in [
        "WITH t AS (SELECT 1) SELECT * FROM t",
        "select\n*\nfrom x",
        "SELECT(1)",
    ] {
        let model = model_with_source(source);
        let generated = generate_sql(&plan, &model, &query(&["REGION"], &[]), &security())
            .expect("generated SQL");
        assert!(
            generated.sql.contains(&format!("FROM ({source})")),
            "{source}: {}",
            generated.sql
        );
    }
}

#[test]
fn e6_4_4_every_node_is_a_cte_in_plan_order() {
    let generated = generate(
        &full_chain(),
        &query(&["REGION"], &["ORDER_COUNT", "TOTAL_AMOUNT"]),
    )
    .expect("generated SQL");
    assert_golden(&generated.sql, include_str!("golden/e6_4_4_plan_order.sql"));
}

#[test]
fn e6_4_8_order_by_states_where_nulls_go() {
    let plan = limit(
        sort(
            scan("ORDERS", &["QUANTITY", "REGION"]),
            &[("REGION", false, true), ("QUANTITY", true, false)],
        ),
        5,
    );
    let generated = generate(&plan, &query(&["QUANTITY", "REGION"], &[])).expect("generated SQL");
    assert_golden(
        &generated.sql,
        include_str!("golden/e6_4_8_order_by_nulls.sql"),
    );
}

#[test]
fn e6_4_9_formatting_is_fixed() {
    let plan = full_chain();
    let columns = query(&["REGION"], &["ORDER_COUNT", "TOTAL_AMOUNT"]);
    let first = generate(&plan, &columns).expect("generated SQL");
    let second = generate(&full_chain(), &columns).expect("generated SQL");
    assert_eq!(first, second);
    assert_golden(&first.sql, include_str!("golden/e6_4_4_plan_order.sql"));
    for line in first.sql.lines() {
        assert_eq!(line, line.trim_end(), "trailing space in {line:?}");
        let indentation = line.len() - line.trim_start().len();
        assert_eq!(indentation % 2, 0, "odd indentation in {line:?}");
        assert!(!line.contains('\t'), "tab in {line:?}");
    }
    let unquoted: String = first.sql.split('"').step_by(2).collect();
    for keyword in [
        "WITH", "SELECT", "FROM", "WHERE", "GROUP BY", "ORDER BY", "LIMIT",
    ] {
        assert!(unquoted.contains(keyword), "{keyword} in upper case");
        assert!(
            !unquoted.contains(&keyword.to_lowercase()),
            "{keyword} in lower case"
        );
    }
    assert!(!first.sql.ends_with('\n'));
}

#[test]
fn e6_5_2_time_grains_truncate_in_the_query_time_zone() {
    let keys = vec![
        named("ORDERED_ON_MONTH", "DATE_TRUNC('month', ORDERED_ON)"),
        named("CREATED_AT_WEEK", "DATE_TRUNC('week', CREATED_AT)"),
        named("CREATED_AT_DAY", "DATE_TRUNC('day', CREATED_AT)"),
    ];
    let plan = aggregate(
        scan("ORDERS", &["CREATED_AT", "ORDERED_ON"]),
        keys,
        vec![component("ORDER_COUNT", "COUNT(*)", None)],
        vec![
            column("ORDERED_ON_MONTH", LogicalType::Date, true),
            column("CREATED_AT_WEEK", support::instant(), true),
            column("CREATED_AT_DAY", support::instant(), true),
            column("ORDER_COUNT", big_integer(), false),
        ],
    );
    let columns = query(
        &["ORDERED_ON_MONTH", "CREATED_AT_WEEK", "CREATED_AT_DAY"],
        &["ORDER_COUNT"],
    );
    let generated = generate(&plan, &columns).expect("generated SQL");
    assert_golden(
        &generated.sql,
        include_str!("golden/e6_5_2_time_grains.sql"),
    );
    assert_eq!(generated.parameters.len(), 1);
    assert_eq!(generated.parameters[0].logical_type, LogicalType::Text);
    assert_eq!(generated.parameters[0].value, json!("Europe/Madrid"));
}

/// The row-policy filters `REGION IN (USER_ATTRIBUTE('regions'))` and
/// `QUANTITY NOT IN (USER_ATTRIBUTE('tenants'))`.
fn list_policies() -> PlanNode {
    let scanned = scan("ORDERS", &["QUANTITY", "REGION"]);
    let regions = filter(
        scanned,
        "REGION IN (USER_ATTRIBUTE('regions'))",
        from_model("/semantic_model/0/datasets/0/row_policies/0"),
    );
    filter(
        regions,
        "QUANTITY NOT IN (USER_ATTRIBUTE('tenants'))",
        from_model("/semantic_model/0/datasets/0/row_policies/1"),
    )
}

#[test]
fn e6_5_3_list_attributes_are_unnested() {
    let generated =
        generate(&list_policies(), &query(&["QUANTITY", "REGION"], &[])).expect("generated SQL");
    assert_golden(
        &generated.sql,
        include_str!("golden/e6_5_3_list_attributes.sql"),
    );
    assert_eq!(generated.parameters[0].value, json!(["north", "south"]));
    assert_eq!(generated.parameters[1].value, json!([1, 2]));
}

#[test]
fn e4_6_9a_empty_lists_keep_the_same_shape() {
    let empty = security_with(r#"{"regions": [], "tenants": []}"#);
    let generated = generate_sql(
        &list_policies(),
        &model(),
        &query(&["QUANTITY", "REGION"], &[]),
        &empty,
    )
    .expect("generated SQL");
    assert_golden(
        &generated.sql,
        include_str!("golden/e6_5_3_list_attributes.sql"),
    );
    assert_eq!(generated.parameters[0].value, json!([]));
    assert_eq!(generated.parameters[1].value, json!([]));
}

#[test]
fn shape_filter_with_parameters() {
    let plan = filter(
        scan("ORDERS", &["AMOUNT", "REGION"]),
        "REGION = 'north' AND AMOUNT > 10.50",
        from_query("/query/filters/0"),
    );
    let generated = generate(&plan, &query(&["AMOUNT", "REGION"], &[])).expect("generated SQL");
    assert_golden(
        &generated.sql,
        include_str!("golden/shape_filter_parameters.sql"),
    );
    assert_eq!(generated.parameters[0].value, json!("north"));
    assert_eq!(generated.parameters[1].logical_type, decimal(4, 2));
    assert_eq!(generated.parameters[1].value, json!("10.50"));
}

#[test]
fn shape_integer_division() {
    let counted = aggregate(
        scan("ORDERS", &["QUANTITY"]),
        Vec::new(),
        vec![
            component("TOTAL_QUANTITY", "SUM(QUANTITY)", None),
            component("ORDER_COUNT", "COUNT(*)", None),
        ],
        vec![
            column("TOTAL_QUANTITY", huge_integer(), true),
            column("ORDER_COUNT", big_integer(), false),
        ],
    );
    let plan = project(
        counted,
        vec![named("AVERAGE_QUANTITY", "TOTAL_QUANTITY / ORDER_COUNT")],
        vec![column("AVERAGE_QUANTITY", double(), true)],
        from_model("/semantic_model/0/metrics/0"),
    );
    let generated = generate(&plan, &query(&[], &["AVERAGE_QUANTITY"])).expect("generated SQL");
    assert_golden(
        &generated.sql,
        include_str!("golden/shape_integer_division.sql"),
    );
}

#[test]
fn shape_decimal_metric_cast() {
    let plan = aggregate(
        scan("ORDERS", &["AMOUNT", "IS_PAID"]),
        Vec::new(),
        vec![
            component("PAID_AMOUNT", "SUM(AMOUNT)", Some("IS_PAID")),
            component("AVERAGE_AMOUNT", "AVG(AMOUNT)", None),
            component("MEDIAN_AMOUNT", "MEDIAN(AMOUNT)", None),
        ],
        vec![
            column("PAID_AMOUNT", decimal(38, 2), true),
            column("AVERAGE_AMOUNT", double(), true),
            column("MEDIAN_AMOUNT", double(), true),
        ],
    );
    let columns = query(&[], &["PAID_AMOUNT", "AVERAGE_AMOUNT", "MEDIAN_AMOUNT"]);
    let generated = generate(&plan, &columns).expect("generated SQL");
    assert_golden(
        &generated.sql,
        include_str!("golden/shape_decimal_metric.sql"),
    );
}

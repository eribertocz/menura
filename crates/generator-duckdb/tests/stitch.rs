//! Tests of the generation of `Stitch` (issue #317): the keys are the distinct key tuples of
//! the branches, compared with `IS NOT DISTINCT FROM`; a missing branch row is detected with a
//! presence column and gives the empty-input value of each component, written as a literal
//! (E6-4.7, E6-4.11, E4-5); the branches are written in branch order (E6-4.12).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::{IntegerBits, LogicalType};
use menura_generator_duckdb::{GeneratedSql, InvariantViolation};
use menura_representations::logical_plan::{Operation, OutputSchema, PlanNode, SchemaColumn};
use menura_representations::names::ColumnName;
use support::{
    aggregate, assert_golden, big_integer, column, component, decimal, filter, from_model,
    from_query, generate, instant, named, query, region_count, scan,
};

/// The sum of 128-bit integers.
fn huge_integer() -> LogicalType {
    LogicalType::Integer {
        bits: IntegerBits::OneHundredTwentyEight,
        signed: true,
    }
}

/// The amount of `ORDERS` per region.
fn region_amount() -> PlanNode {
    aggregate(
        scan("ORDERS", &["AMOUNT", "REGION"]),
        vec![named("REGION", "REGION")],
        vec![component("TOTAL_AMOUNT", "SUM(AMOUNT)", None)],
        vec![
            column("REGION", LogicalType::Text, true),
            column("TOTAL_AMOUNT", decimal(38, 2), true),
        ],
    )
}

/// The number of distinct quantities of `ORDERS` per region.
fn region_quantities() -> PlanNode {
    aggregate(
        scan("ORDERS", &["QUANTITY", "REGION"]),
        vec![named("REGION", "REGION")],
        vec![component(
            "DISTINCT_QUANTITIES",
            "COUNT(DISTINCT QUANTITY)",
            None,
        )],
        vec![
            column("REGION", LogicalType::Text, true),
            column("DISTINCT_QUANTITIES", big_integer(), false),
        ],
    )
}

/// The `Stitch` of `branches` by `keys`, producing `schema`.
fn stitch(branches: Vec<PlanNode>, keys: &[&str], schema: Vec<SchemaColumn>) -> PlanNode {
    PlanNode::new(
        Operation::Stitch {
            branches,
            keys: keys.iter().map(|key| ColumnName::new(key)).collect(),
        },
        OutputSchema::new(schema),
        from_model("/semantic_model/0/metrics/0"),
    )
}

/// The columns of the stitch of `region_count` and `region_amount`.
fn two_branch_columns() -> Vec<SchemaColumn> {
    vec![
        column("REGION", LogicalType::Text, true),
        column("ORDER_COUNT", big_integer(), false),
        column("TOTAL_QUANTITY", huge_integer(), true),
        column("TOTAL_AMOUNT", decimal(38, 2), true),
    ]
}

/// The SQL of `plan` for the query `REGION` then `metrics`.
fn generated(plan: &PlanNode, metrics: &[&str]) -> GeneratedSql {
    generate(plan, &query(&["REGION"], metrics)).expect("SQL")
}

fn two_branches() -> PlanNode {
    stitch(
        vec![region_count(), region_amount()],
        &["REGION"],
        two_branch_columns(),
    )
}

const TWO_METRICS: [&str; 3] = ["ORDER_COUNT", "TOTAL_QUANTITY", "TOTAL_AMOUNT"];

#[test]
fn e6_4_7_stitch_of_two_branches() {
    assert_golden(
        &generated(&two_branches(), &TWO_METRICS).sql,
        include_str!("golden/e6_4_7_stitch_two_branches.sql"),
    );
}

#[test]
fn e6_4_12_stitch_of_three_branches_in_branch_order() {
    let mut columns = two_branch_columns();
    columns.push(column("DISTINCT_QUANTITIES", big_integer(), false));
    let plan = stitch(
        vec![region_count(), region_amount(), region_quantities()],
        &["REGION"],
        columns,
    );
    let sql = generated(
        &plan,
        &[
            "ORDER_COUNT",
            "TOTAL_QUANTITY",
            "TOTAL_AMOUNT",
            "DISTINCT_QUANTITIES",
        ],
    )
    .sql;
    assert_golden(
        &sql,
        include_str!("golden/e6_4_12_stitch_three_branches.sql"),
    );
    let position = |name: &str| sql.find(name).expect(name);
    assert!(position(r#""menura_2_aggregate""#) < position(r#""menura_4_aggregate""#));
    assert!(position(r#""menura_4_aggregate""#) < position(r#""menura_6_aggregate""#));
    assert!(position(r#""menura_6_aggregate""#) < position(r#""menura_7_stitch""#));
}

#[test]
fn e6_4_11_stitch_member_missing_in_one_branch() {
    // Only the north is counted: every other region of the amounts has no row in the first
    // branch, so its count is the empty-input value `0`, a literal, and its sum is NULL.
    let north = aggregate(
        filter(
            scan("ORDERS", &["QUANTITY", "REGION"]),
            "REGION = 'north'",
            from_query("/query/filters/0"),
        ),
        vec![named("REGION", "REGION")],
        vec![
            component("ORDER_COUNT", "COUNT(*)", None),
            component("TOTAL_QUANTITY", "SUM(QUANTITY)", None),
        ],
        vec![
            column("REGION", LogicalType::Text, true),
            column("ORDER_COUNT", big_integer(), false),
            column("TOTAL_QUANTITY", huge_integer(), true),
        ],
    );
    let plan = stitch(
        vec![north, region_amount()],
        &["REGION"],
        two_branch_columns(),
    );
    let generated = generated(&plan, &TWO_METRICS);
    assert_golden(
        &generated.sql,
        include_str!("golden/e6_4_11_stitch_missing_member.sql"),
    );
    assert_eq!(generated.parameters.len(), 1);
    assert!(
        generated.sql.contains(
            r#"CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."ORDER_COUNT" ELSE CAST(0 AS BIGINT) END AS "ORDER_COUNT""#
        ),
        "{}",
        generated.sql
    );
}

#[test]
fn e6_4_7_stitch_null_keys_are_equal() {
    let by_region_and_month = |input: PlanNode, name: &str, call: &str, result: LogicalType| {
        aggregate(
            input,
            vec![
                named("REGION", "REGION"),
                named("CREATED_MONTH", "CREATED_MONTH"),
            ],
            vec![component(name, call, None)],
            vec![
                column("REGION", LogicalType::Text, true),
                column("CREATED_MONTH", instant(), true),
                column(name, result, true),
            ],
        )
    };
    let plan = stitch(
        vec![
            by_region_and_month(
                scan("ORDERS", &["QUANTITY", "REGION", "CREATED_MONTH"]),
                "TOTAL_QUANTITY",
                "SUM(QUANTITY)",
                huge_integer(),
            ),
            by_region_and_month(
                scan("ORDERS", &["AMOUNT", "REGION", "CREATED_MONTH"]),
                "TOTAL_AMOUNT",
                "SUM(AMOUNT)",
                decimal(38, 2),
            ),
        ],
        &["REGION", "CREATED_MONTH"],
        vec![
            column("REGION", LogicalType::Text, true),
            column("CREATED_MONTH", instant(), true),
            column("TOTAL_QUANTITY", huge_integer(), true),
            column("TOTAL_AMOUNT", decimal(38, 2), true),
        ],
    );
    let sql = generate(
        &plan,
        &query(
            &["REGION", "CREATED_MONTH"],
            &["TOTAL_QUANTITY", "TOTAL_AMOUNT"],
        ),
    )
    .expect("SQL")
    .sql;
    assert_golden(&sql, include_str!("golden/e6_4_7_stitch_null_keys.sql"));
    // A NULL key is one key (E4-5.0a, E4-5.1): `UNION` and `IS NOT DISTINCT FROM` compare
    // NULLs as equal, and no value that can be NULL is coalesced.
    assert!(sql.contains(" UNION SELECT "), "{sql}");
    assert!(!sql.contains("UNION ALL"), "{sql}");
    assert!(!sql.contains("COALESCE"), "{sql}");
    assert_eq!(sql.matches(" IS NOT DISTINCT FROM ").count(), 4, "{sql}");
}

#[test]
fn e4_5_2_stitch_without_keys_joins_one_row_per_branch() {
    let total = |input: PlanNode, name: &str, call: &str, result: LogicalType| {
        aggregate(
            input,
            Vec::new(),
            vec![component(name, call, None)],
            vec![column(name, result, false)],
        )
    };
    let plan = stitch(
        vec![
            total(
                scan("ORDERS", &[]),
                "ORDER_COUNT",
                "COUNT(*)",
                big_integer(),
            ),
            total(
                scan("ORDERS", &["AMOUNT"]),
                "TOTAL_AMOUNT",
                "SUM(AMOUNT)",
                decimal(38, 2),
            ),
        ],
        &[],
        vec![
            column("ORDER_COUNT", big_integer(), false),
            column("TOTAL_AMOUNT", decimal(38, 2), true),
        ],
    );
    let sql = generate(&plan, &query(&[], &["ORDER_COUNT", "TOTAL_AMOUNT"]))
        .expect("SQL")
        .sql;
    assert_golden(&sql, include_str!("golden/e4_5_2_stitch_without_keys.sql"));
}

#[test]
fn pln_1_stitched_columns_keep_the_nullability_of_the_stitch() {
    let generated = generated(&two_branches(), &TWO_METRICS);
    let nullable: Vec<(&str, bool)> = generated
        .result_schema
        .iter()
        .map(|column| (column.name.as_str(), column.nullable))
        .collect();
    assert_eq!(
        nullable,
        vec![
            ("REGION", true),
            ("ORDER_COUNT", false),
            ("TOTAL_QUANTITY", true),
            ("TOTAL_AMOUNT", true),
        ]
    );
}

#[test]
fn e6_4_12_presence_column_never_collides_with_a_branch_column() {
    let present = aggregate(
        scan("ORDERS", &["QUANTITY", "REGION"]),
        vec![named("REGION", "REGION")],
        vec![component("menura.present", "SUM(QUANTITY)", None)],
        vec![
            column("REGION", LogicalType::Text, true),
            column("menura.present", huge_integer(), true),
        ],
    );
    let plan = stitch(
        vec![present, region_amount()],
        &["REGION"],
        vec![
            column("REGION", LogicalType::Text, true),
            column("menura.present", huge_integer(), true),
            column("TOTAL_AMOUNT", decimal(38, 2), true),
        ],
    );
    let sql = generated(&plan, &["menura.present", "TOTAL_AMOUNT"]).sql;
    assert!(
        sql.contains(r#"TRUE AS "menura.present." FROM "menura_2_aggregate""#),
        "{sql}"
    );
}

#[test]
fn e4_3_7_stitch_key_missing_in_a_branch_is_an_error() {
    let plan = stitch(
        vec![region_count(), region_amount()],
        &["CREATED_MONTH"],
        two_branch_columns(),
    );
    let error = generate(&plan, &query(&["REGION"], &TWO_METRICS)).expect_err("unknown key");
    assert_eq!(error.violation, InvariantViolation::UnknownColumn);
}

#[test]
fn e4_3_7_stitch_column_of_no_branch_is_an_error() {
    let mut columns = two_branch_columns();
    columns.push(column("MISSING", big_integer(), true));
    let plan = stitch(vec![region_count(), region_amount()], &["REGION"], columns);
    let error = generate(
        &plan,
        &query(
            &["REGION"],
            &["ORDER_COUNT", "TOTAL_QUANTITY", "TOTAL_AMOUNT", "MISSING"],
        ),
    )
    .expect_err("unknown column");
    assert_eq!(error.violation, InvariantViolation::UnknownColumn);
}

#[test]
fn e4_3_7_stitch_column_of_two_branches_is_an_error() {
    let plan = stitch(
        vec![region_count(), region_count()],
        &["REGION"],
        vec![
            column("REGION", LogicalType::Text, true),
            column("ORDER_COUNT", big_integer(), false),
            column("TOTAL_QUANTITY", huge_integer(), true),
        ],
    );
    let error = generate(&plan, &query(&["REGION"], &["ORDER_COUNT", "TOTAL_QUANTITY"]))
        .expect_err("ambiguous column");
    assert_eq!(error.violation, InvariantViolation::AmbiguousStitchColumn);
}

#[test]
fn e4_5_0b_stitch_branch_without_aggregate_is_an_error() {
    let plan = stitch(
        vec![scan("ORDERS", &["REGION", "QUANTITY"]), region_amount()],
        &["REGION"],
        vec![
            column("REGION", LogicalType::Text, true),
            column("QUANTITY", big_integer(), true),
            column("TOTAL_AMOUNT", decimal(38, 2), true),
        ],
    );
    let error = generate(&plan, &query(&["REGION"], &["QUANTITY", "TOTAL_AMOUNT"]))
        .expect_err("no empty-input value");
    assert_eq!(error.violation, InvariantViolation::UnknownEmptyInputValue);
}

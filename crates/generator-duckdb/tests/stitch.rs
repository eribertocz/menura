//! Tests of the generation of `Stitch` (issue #317): the keys are the distinct key tuples of
//! the branches, compared with `IS NOT DISTINCT FROM`; a missing branch row is detected with a
//! presence column and gives the empty-input value of each component, written as a literal
//! (E6-4.7, E6-4.11, E4-5); the branches are written in branch order (E6-4.12).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;
use menura_contracts::types::common::LogicalType;
use menura_representations::logical_plan::PlanNode;
use support::{
    TWO_METRICS, aggregate, assert_golden, big_integer, column, component, decimal, filter,
    from_query, generate, generated, huge_integer, instant, named, query, region_amount,
    region_count, region_quantities, scan, stitch, two_branch_columns, two_branches,
};

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
        &[0, 0, 1, 2],
        &["ORDER_COUNT", "DISTINCT_QUANTITIES"],
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
    assert!(position(r#""menura_6_aggregate""#) < position(r#""menura_11_stitch""#));
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
        &[0, 0, 1],
        &["ORDER_COUNT"],
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

/// The stitch of the quantities and the amounts by region and month.
fn by_region_and_month() -> PlanNode {
    let aggregate_by = |input: PlanNode, name: &str, call: &str, result: LogicalType| {
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
    stitch(
        vec![
            aggregate_by(
                scan("ORDERS", &["QUANTITY", "REGION", "CREATED_MONTH"]),
                "TOTAL_QUANTITY",
                "SUM(QUANTITY)",
                huge_integer(),
            ),
            aggregate_by(
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
        &[0, 1],
        &[],
    )
}

#[test]
fn e6_4_7_stitch_null_keys_are_equal() {
    let plan = by_region_and_month();
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
    assert!(sql.contains("\n    UNION\n"), "{sql}");
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
        &[0, 1],
        &["ORDER_COUNT"],
    );
    let sql = generate(&plan, &query(&[], &["ORDER_COUNT", "TOTAL_AMOUNT"]))
        .expect("SQL")
        .sql;
    assert_golden(&sql, include_str!("golden/e4_5_2_stitch_without_keys.sql"));
}

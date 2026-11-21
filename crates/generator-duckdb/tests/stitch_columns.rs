//! Tests of the columns of a generated `Stitch` (issue #317): the result schema keeps the
//! nullability of the `Stitch` (PLN-1), the presence column never collides with a branch column
//! (E6-4.12), and a `Stitch` whose columns do not match its branches is an internal error.

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::LogicalType;
use menura_generator_duckdb::InvariantViolation;
use menura_representations::logical_plan::{OutputSchema, PlanNode};
use support::{
    TWO_METRICS, aggregate, big_integer, column, component, decimal, from_model, generate,
    generated, huge_integer, named, query, region_amount, region_count, scan, stitch,
    two_branch_columns, two_branches,
};

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
        &[0, 1],
        &[],
    );
    let sql = generated(&plan, &["menura.present", "TOTAL_AMOUNT"]).sql;
    assert!(sql.contains(r#"TRUE AS "menura.present.""#), "{sql}");
}

#[test]
fn e4_3_7_stitch_key_missing_in_a_branch_is_an_error() {
    let plan = stitch(
        vec![region_count(), region_amount()],
        &["CREATED_MONTH"],
        two_branch_columns(),
        &[0, 0, 1],
        &["ORDER_COUNT"],
    );
    let error = generate(&plan, &query(&["REGION"], &TWO_METRICS)).expect_err("unknown key");
    assert_eq!(error.violation, InvariantViolation::UnknownColumn);
}

#[test]
fn e4_3_7_stitch_column_of_no_branch_is_an_error() {
    let mut columns = two_branch_columns();
    columns.push(column("MISSING", big_integer(), true));
    let plan = stitch(
        vec![region_count(), region_amount()],
        &["REGION"],
        columns,
        &[0, 0, 1, 1],
        &["ORDER_COUNT"],
    );
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
fn e4_3_7_stitch_column_of_an_unknown_branch_is_an_error() {
    let plan = stitch(
        vec![region_count(), region_amount()],
        &["REGION"],
        two_branch_columns(),
        &[0, 0, 2],
        &["ORDER_COUNT"],
    );
    let error = generate(&plan, &query(&["REGION"], &TWO_METRICS)).expect_err("unknown branch");
    assert_eq!(error.violation, InvariantViolation::UnknownBranch);
}

#[test]
fn e4_3_7_stitch_schema_out_of_column_order_is_an_error() {
    let mut columns = two_branch_columns();
    columns.swap(1, 3);
    let plan = PlanNode::new(
        two_branches().operation().clone(),
        OutputSchema::new(columns),
        from_model("/semantic_model/0/metrics/0"),
    );
    let error = generate(
        &plan,
        &query(
            &["REGION"],
            &["TOTAL_AMOUNT", "TOTAL_QUANTITY", "ORDER_COUNT"],
        ),
    )
    .expect_err("schema out of order");
    assert_eq!(error.violation, InvariantViolation::UnknownColumn);
}

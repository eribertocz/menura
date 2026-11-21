//! Golden SQL of `Join` and `Deduplicate` (E6-4.4, E6-4.12, E6-4.13, E6-4.14) and the
//! nullability of joined columns (PLN-1).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::LogicalType;
use menura_generator_duckdb::InvariantViolation;
use menura_representations::logical_plan::PlanNode;
use support::{
    ROW_IDENTITY, aggregate, assert_golden, big_integer, column, component, decimal, deduplicate,
    generate, generate_all, home_scan, home_scan_with_identity, join_of, joined_scan, left_join,
    named, query,
};

const STAGED: &str = "ORDERS_STAGED.";

/// `ORDERS` left joined to its `STAGED` instance on `QUANTITY`.
fn one_join() -> PlanNode {
    left_join(
        home_scan("ORDERS", &["QUANTITY", "REGION"]),
        joined_scan("STAGED", STAGED, &["QUANTITY", "REGION"]),
        "ORDERS_STAGED",
        &[("QUANTITY", "ORDERS_STAGED.QUANTITY")],
    )
}

#[test]
fn e6_4_12_one_join() {
    let generated = generate_all(&one_join());
    assert_golden(&generated.sql, include_str!("golden/e6_4_12_one_join.sql"));
}

#[test]
fn e6_4_12_a_chain_of_joins_is_written_in_post_order() {
    let chained = "ORDERS_STAGED.STAGED_ORDERS.";
    let plan = left_join(
        one_join(),
        joined_scan("ORDERS", chained, &["REGION", "UNITS"]),
        "STAGED_ORDERS",
        &[("ORDERS_STAGED.REGION", "ORDERS_STAGED.STAGED_ORDERS.REGION")],
    );
    let generated = generate_all(&plan);
    assert_golden(
        &generated.sql,
        include_str!("golden/e6_4_12_join_chain.sql"),
    );
}

#[test]
fn e6_4_12_role_playing_instances_are_two_scans() {
    let plan = left_join(
        left_join(
            home_scan("ORDERS", &["QUANTITY", "UNITS"]),
            joined_scan("STAGED", "BILLING.", &["QUANTITY", "REGION"]),
            "BILLING",
            &[("QUANTITY", "BILLING.QUANTITY")],
        ),
        joined_scan("STAGED", "SHIPPING.", &["UNITS", "REGION"]),
        "SHIPPING",
        &[("UNITS", "SHIPPING.UNITS")],
    );
    let generated = generate_all(&plan);
    assert_golden(
        &generated.sql,
        include_str!("golden/e6_4_12_role_playing.sql"),
    );
}

#[test]
fn e6_4_13_keys_of_different_types_are_cast_to_their_common_type() {
    let plan = left_join(
        home_scan("ORDERS", &["QUANTITY"]),
        joined_scan("STAGED", STAGED, &["AMOUNT", "REGION"]),
        "ORDERS_STAGED",
        &[("QUANTITY", "ORDERS_STAGED.AMOUNT")],
    );
    let generated = generate_all(&plan);
    assert_golden(&generated.sql, include_str!("golden/e6_4_13_key_cast.sql"));
}

#[test]
fn e6_4_14_deduplicate_with_a_grain() {
    let joined = left_join(
        home_scan_with_identity("ORDERS", &["QUANTITY", "CREATED_AT"]),
        joined_scan("STAGED", STAGED, &["QUANTITY", "REGION"]),
        "ORDERS_STAGED",
        &[("QUANTITY", "ORDERS_STAGED.QUANTITY")],
    );
    let group = [
        "DATE_TRUNC('month', CREATED_AT)",
        "\"ORDERS_STAGED.REGION\"",
    ];
    let plan = aggregate(
        deduplicate(joined, &group, &[ROW_IDENTITY]),
        vec![
            named("CREATED_MONTH", group[0]),
            named("ORDERS_STAGED.REGION", group[1]),
        ],
        vec![component("TOTAL_QUANTITY", "SUM(QUANTITY)", None)],
        vec![
            column("CREATED_MONTH", support::instant(), true),
            column("ORDERS_STAGED.REGION", LogicalType::Text, true),
            column("TOTAL_QUANTITY", decimal(38, 0), true),
        ],
    );
    let dimensions = ["CREATED_MONTH", "ORDERS_STAGED.REGION"];
    let generated = generate(&plan, &query(&dimensions, &["TOTAL_QUANTITY"])).expect("SQL");
    assert_golden(
        &generated.sql,
        include_str!("golden/e6_4_14_deduplicate_grain.sql"),
    );
}

#[test]
fn pln_1_columns_of_the_right_side_of_a_left_join_are_nullable() {
    let generated = generate_all(&one_join());
    let nullable: Vec<(&str, bool)> = generated
        .result_schema
        .iter()
        .map(|column| (column.name.as_str(), column.nullable))
        .collect();
    assert_eq!(
        nullable,
        [
            ("QUANTITY", true),
            ("REGION", true),
            ("ORDERS_STAGED.QUANTITY", true),
            ("ORDERS_STAGED.REGION", true),
        ]
    );
}

#[test]
fn pln_1_a_non_nullable_right_column_of_a_left_join_is_an_error() {
    let left = home_scan("ORDERS", &["QUANTITY"]);
    let right = joined_scan("STAGED", STAGED, &["QUANTITY"]);
    let mut columns = left.schema().columns().to_vec();
    columns.push(column("ORDERS_STAGED.QUANTITY", big_integer(), false));
    let plan = join_of(
        left,
        right,
        ("ORDERS_STAGED", STAGED),
        &[("QUANTITY", "ORDERS_STAGED.QUANTITY")],
        columns,
    );
    let error = generate(&plan, &query(&["QUANTITY", "ORDERS_STAGED.QUANTITY"], &[]))
        .expect_err("a non-nullable right column");
    assert_eq!(error.violation, InvariantViolation::NonNullableJoinedColumn);
}

#[test]
fn e6_4_13_keys_without_a_common_type_are_an_error() {
    let plan = left_join(
        home_scan("ORDERS", &["QUANTITY"]),
        joined_scan("STAGED", STAGED, &["REGION"]),
        "ORDERS_STAGED",
        &[("QUANTITY", "ORDERS_STAGED.REGION")],
    );
    let names = ["QUANTITY", "ORDERS_STAGED.REGION"];
    let error = generate(&plan, &query(&names, &[])).expect_err("incompatible keys");
    assert_eq!(error.violation, InvariantViolation::IncompatibleJoinKeys);
}

#[test]
fn e6_4_12_a_join_without_keys_is_an_error() {
    let plan = left_join(
        home_scan("ORDERS", &["QUANTITY"]),
        joined_scan("STAGED", STAGED, &["REGION"]),
        "ORDERS_STAGED",
        &[],
    );
    let names = ["QUANTITY", "ORDERS_STAGED.REGION"];
    let error = generate(&plan, &query(&names, &[])).expect_err("no keys");
    assert_eq!(error.violation, InvariantViolation::JoinWithoutKeys);
}

#[test]
fn e6_4_12_a_joined_scan_column_outside_its_instance_is_an_error() {
    let plan = left_join(
        home_scan("ORDERS", &["QUANTITY"]),
        joined_with_bare_column(),
        "ORDERS_STAGED",
        &[("QUANTITY", "ORDERS_STAGED.QUANTITY")],
    );
    let names = ["QUANTITY", "ORDERS_STAGED.QUANTITY", "REGION"];
    let error = generate(&plan, &query(&names, &[])).expect_err("a bare column");
    assert_eq!(error.violation, InvariantViolation::UnknownField);
}

/// The `STAGED` instance `orders_staged` with a column `REGION` that lacks its prefix.
fn joined_with_bare_column() -> PlanNode {
    let scan = joined_scan("STAGED", STAGED, &["QUANTITY"]);
    let mut columns = scan.schema().columns().to_vec();
    columns.push(column("REGION", LogicalType::Text, true));
    PlanNode::new(
        scan.operation().clone(),
        menura_representations::logical_plan::OutputSchema::new(columns),
        scan.origin().clone(),
    )
}

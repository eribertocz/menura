//! Tests of the `Snapshot` node (issue #358): the rows of its input reduced to one value per
//! (result group, entity) under a semi-additive rule (E4-3.13, E4-7), with the runtime errors
//! of ADD-7 raised in SQL (E6-4.6b) and the SQL shape fixed by golden files (E6-4.14).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::LogicalType;
use menura_generator_duckdb::InvariantViolation;
use menura_model::internal_model::{NormalizedName, NullHandling, Selection, SnapshotAggregation};
use menura_representations::logical_plan::{
    Operation, OutputSchema, PlanNode, SnapshotOver, SnapshotRule,
};
use support::{
    assert_golden, big_integer, column, component, from_model, generate_billing, named, parse,
    population_scan, query,
};

const STOCK_FIELDS: [&str; 4] = ["WAREHOUSE", "PRODUCT_ID", "CATEGORY", "QUANTITY"];

/// The rule of `STOCK_LEVEL`: `SUM(QUANTITY)` per `CATEGORY` and `PRODUCT_ID`, across the
/// field `WAREHOUSE`.
fn stock_rule(aggregation: SnapshotAggregation, nulls: NullHandling) -> SnapshotRule {
    SnapshotRule {
        metric: NormalizedName::new("STOCK_LEVEL"),
        keys: vec![named("CATEGORY", "CATEGORY")],
        entity: vec![named("PRODUCT_ID", "PRODUCT_ID")],
        components: vec![component("QUANTITY", "SUM(QUANTITY)", None)],
        aggregation,
        nulls,
        over: SnapshotOver::Field(parse("WAREHOUSE")),
    }
}

fn stock_snapshot(rule: SnapshotRule) -> PlanNode {
    PlanNode::new(
        Operation::Snapshot {
            input: Box::new(population_scan("STOCK", &STOCK_FIELDS)),
            rule,
        },
        OutputSchema::new(vec![
            column("CATEGORY", LogicalType::Text, true),
            column("PRODUCT_ID", big_integer(), true),
            column("QUANTITY", big_integer(), true),
        ]),
        from_model("/semantic_model/0/metrics/0"),
    )
}

#[cfg(test)]
fn stock_sql(aggregation: SnapshotAggregation, nulls: NullHandling) -> String {
    let plan = stock_snapshot(stock_rule(aggregation, nulls));
    generate_billing(&plan, &query(&["CATEGORY", "PRODUCT_ID"], &["QUANTITY"]))
        .expect("SQL")
        .sql
}

#[test]
fn e4_7_1_field_over_maximum_keeping_nulls() {
    assert_golden(
        &stock_sql(SnapshotAggregation::Maximum, NullHandling::Keep),
        include_str!("golden/e4_7_1_snapshot_field_maximum.sql"),
    );
}

#[test]
fn e4_7_2_field_over_average_with_null_as_zero() {
    assert_golden(
        &stock_sql(SnapshotAggregation::Average, NullHandling::Zero),
        include_str!("golden/e4_7_2_snapshot_field_average_zero.sql"),
    );
}

#[test]
fn radd_03_field_over_null_value_raises_in_sql() {
    let sql = stock_sql(SnapshotAggregation::Minimum, NullHandling::Error);
    assert_golden(
        &sql,
        include_str!("golden/radd_03_snapshot_field_null_error.sql"),
    );
    assert!(
        sql.contains(
            "error('RADD-03: nulls: error and a NULL value exists in metric STOCK_LEVEL')"
        )
    );
}

#[test]
fn e4_7_1_first_or_last_across_a_field_is_an_invariant_violation() {
    let plan = stock_snapshot(stock_rule(
        SnapshotAggregation::Last(Selection::GlobalPeriod),
        NullHandling::Keep,
    ));
    let error = generate_billing(&plan, &query(&["CATEGORY", "PRODUCT_ID"], &["QUANTITY"]))
        .expect_err("no time order");
    assert_eq!(error.violation, InvariantViolation::SnapshotOutsideRule);
}

#[test]
fn e4_7_1_null_as_missing_across_a_field_is_an_invariant_violation() {
    let plan = stock_snapshot(stock_rule(
        SnapshotAggregation::Maximum,
        NullHandling::AsMissing,
    ));
    let error = generate_billing(&plan, &query(&["CATEGORY", "PRODUCT_ID"], &["QUANTITY"]))
        .expect_err("no missing without time");
    assert_eq!(error.violation, InvariantViolation::SnapshotOutsideRule);
}

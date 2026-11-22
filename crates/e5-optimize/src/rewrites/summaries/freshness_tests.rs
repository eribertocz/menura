//! Unit tests of summary freshness, conditions 1 to 3 of CAT-7.2.

use menura_model::expression::{Expression, Literal};
use serde_json::json;

use super::freshness::FailedCondition;
use super::freshness_fixture::{
    Fixture, base, build, date_type, field, fresh_state, freshness, instant_type, range, token,
    versions,
};

#[test]
fn cat_7_2_a_month_summary_over_day_partitions_is_valid() {
    assert_eq!(
        freshness(&Fixture::new(), fresh_state(), "UTC", None),
        Ok(())
    );
}

#[test]
fn cat_7_2_condition_1_fails_when_the_base_is_not_partitioned() {
    let mut state = fresh_state();
    *base(&mut state) =
        json!({"source": "shop.main.orders", "row_count_estimate": 10, "version": token("v1")});
    let result = freshness(&Fixture::new(), state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::Partitions));
}

#[test]
fn cat_7_2_condition_1_fails_when_the_summary_grain_does_not_derive_from_the_partitions() {
    let mut state = fresh_state();
    base(&mut state)["partitions"]["grain"] = json!("week");
    base(&mut state)["partitions"]["versions"] = versions(&["2026-W38"]);
    let result = freshness(&Fixture::new(), state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::Partitions));
}

#[test]
fn cat_7_2_condition_1_fails_when_the_partitions_are_on_another_column() {
    let mut state = fresh_state();
    base(&mut state)["partitions"]["column"] = json!("SHIPPED_DATE");
    let result = freshness(&Fixture::new(), state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::Partitions));
}

#[test]
fn cat_7_2_condition_1_fails_when_the_coverage_field_is_an_expression() {
    let mut fixture = Fixture::new();
    fixture.order_date = field(
        "ORDER_DATE",
        Expression::Literal(Literal::Boolean(true)),
        date_type(),
    );
    let result = freshness(&fixture, fresh_state(), "UTC", None);
    assert_eq!(result, Err(FailedCondition::Partitions));
}

#[test]
fn cat_7_2_condition_1_fails_when_the_partition_column_has_a_time_zone() {
    let mut fixture = Fixture::new();
    fixture.column_type = instant_type();
    let result = freshness(&fixture, fresh_state(), "UTC", None);
    assert_eq!(result, Err(FailedCondition::Partitions));
}

#[test]
fn cat_7_2_condition_1_a_nullable_partition_column_needs_a_date_range_on_coverage() {
    let mut fixture = Fixture::new();
    fixture.nullable = true;
    let result = freshness(&fixture, fresh_state(), "UTC", None);
    assert_eq!(result, Err(FailedCondition::Partitions));

    let on_other = range("SHIPPED_DATE", (2026, 9, 1), (2026, 10, 31));
    let result = freshness(&fixture, fresh_state(), "UTC", Some(&on_other));
    assert_eq!(result, Err(FailedCondition::Partitions));

    let on_coverage = range("ORDER_DATE", (2026, 9, 1), (2026, 10, 31));
    let result = freshness(&fixture, fresh_state(), "UTC", Some(&on_coverage));
    assert_eq!(result, Ok(()));
}

#[test]
fn cat_7_2_condition_2_fails_when_a_period_is_outside_coverage() {
    let mut state = fresh_state();
    build(&mut state)["coverage"] = json!({"from": "2026-10", "to": "2026-10"});
    let result = freshness(&Fixture::new(), state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::Coverage));
}

#[test]
fn cat_7_2_condition_3_fails_when_a_base_period_is_added_after_the_build() {
    let mut state = fresh_state();
    base(&mut state)["partitions"]["versions"] =
        versions(&["2026-09-15", "2026-09-20", "2026-10-02", "2026-10-31"]);
    let result = freshness(&Fixture::new(), state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::BuiltFrom));
}

#[test]
fn cat_7_2_condition_3_fails_when_a_base_version_changed() {
    let mut state = fresh_state();
    base(&mut state)["partitions"]["versions"][1]["version"] = token("rewritten");
    let result = freshness(&Fixture::new(), state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::BuiltFrom));
}

#[test]
fn cat_7_2_condition_3_fails_when_built_from_has_another_grain() {
    let mut state = fresh_state();
    build(&mut state)["built_from"] = versions(&["2026-09", "2026-10"]);
    let result = freshness(&Fixture::new(), state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::BuiltFrom));
}

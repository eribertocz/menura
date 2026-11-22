//! Unit tests of summary freshness, conditions 4 to 6 of CAT-7.2, CAT-7.2.1 and the set of
//! periods P (SUM-3.7).

use menura_contracts::types::common::Grain;
use serde_json::json;

use super::freshness::FailedCondition;
use super::freshness_fixture::{
    Fixture, base, build, column_field, dimension, fresh_state, freshness, instant_type, range,
    versions,
};
use crate::test_support::OTHER_HASH;

#[test]
fn cat_7_2_condition_4_fails_when_the_definition_changed() {
    let mut state = fresh_state();
    build(&mut state)["definition_hash"] = json!(OTHER_HASH);
    let result = freshness(&Fixture::new(), state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::DefinitionHash));
}

#[test]
fn cat_7_2_condition_5_fails_when_an_instant_dimension_was_built_in_another_zone() {
    let mut fixture = Fixture::new();
    fixture
        .extra_fields
        .push(column_field("ORDERED_AT", instant_type()));
    fixture
        .extra_dimensions
        .push(dimension("ORDERED_DAY", "ORDERED_AT", Some(Grain::Day)));
    let mut state = fresh_state();
    build(&mut state)["time_zone"] = json!("Europe/Madrid");
    let result = freshness(&fixture, state.clone(), "UTC", None);
    assert_eq!(result, Err(FailedCondition::TimeZone));
    assert_eq!(freshness(&fixture, state, "Europe/Madrid", None), Ok(()));
}

#[test]
fn cat_7_2_condition_5_compares_time_zones_by_code_points() {
    let mut fixture = Fixture::new();
    fixture
        .extra_fields
        .push(column_field("ORDERED_AT", instant_type()));
    fixture
        .extra_dimensions
        .push(dimension("ORDERED_DAY", "ORDERED_AT", Some(Grain::Day)));
    let result = freshness(&fixture, fresh_state(), "Etc/UTC", None);
    assert_eq!(result, Err(FailedCondition::TimeZone));
}

#[test]
fn cat_7_2_condition_5_holds_without_an_instant_dimension() {
    let mut state = fresh_state();
    build(&mut state)["time_zone"] = json!("Europe/Madrid");
    assert_eq!(freshness(&Fixture::new(), state, "UTC", None), Ok(()));
}

#[test]
fn cat_7_2_condition_6_fails_when_the_check_did_not_pass() {
    let mut state = fresh_state();
    build(&mut state)["integrity_check"]["status"] = json!("failed");
    let result = freshness(&Fixture::new(), state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::IntegrityCheck));
}

#[test]
fn cat_7_2_condition_6_fails_when_the_check_ran_on_another_definition() {
    let mut state = fresh_state();
    build(&mut state)["integrity_check"]["definition_hash"] = json!(OTHER_HASH);
    let result = freshness(&Fixture::new(), state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::IntegrityCheck));
}

#[test]
fn cat_7_2_condition_6_fails_when_the_check_missed_a_base_period() {
    let mut state = fresh_state();
    build(&mut state)["integrity_check"]["verified"] = versions(&["2026-09-15", "2026-10-02"]);
    let result = freshness(&Fixture::new(), state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::IntegrityCheck));
}

#[test]
fn cat_7_2_1_a_period_listed_by_neither_side_needs_no_evidence() {
    let mut state = fresh_state();
    // 2026-09-16 is in Q (inside `2026-09`) and listed by no side; so is all of a later month
    // that no side lists.
    build(&mut state)["coverage"] = json!({"from": "2026-09", "to": "2026-10"});
    let query = range("ORDER_DATE", (2026, 9, 1), (2026, 12, 31));
    let result = freshness(&Fixture::new(), state, "UTC", Some(&query));
    assert_eq!(result, Ok(()));
}

#[test]
fn cat_7_2_a_date_range_on_coverage_restricts_p() {
    let mut state = fresh_state();
    let later = ["2026-09-15", "2026-10-02", "2026-10-31", "2026-11-03"];
    base(&mut state)["partitions"]["versions"] = versions(&later);
    assert_eq!(
        freshness(&Fixture::new(), state.clone(), "UTC", None),
        Err(FailedCondition::Coverage)
    );

    let on_other = range("SHIPPED_DATE", (2026, 9, 1), (2026, 10, 31));
    assert_eq!(
        freshness(&Fixture::new(), state.clone(), "UTC", Some(&on_other)),
        Err(FailedCondition::Coverage)
    );

    let on_coverage = range("ORDER_DATE", (2026, 9, 1), (2026, 10, 31));
    let result = freshness(&Fixture::new(), state, "UTC", Some(&on_coverage));
    assert_eq!(result, Ok(()));
}

#[test]
fn cat_7_2_a_week_summary_groups_day_partitions_into_iso_weeks() {
    let mut fixture = Fixture::new();
    fixture.coverage_grain = Grain::Week;
    let mut state = fresh_state();
    let days = ["2021-01-03", "2021-01-04"];
    base(&mut state)["partitions"]["versions"] = versions(&days);
    build(&mut state)["built_from"] = versions(&days);
    build(&mut state)["integrity_check"]["verified"] = versions(&days);
    build(&mut state)["coverage"] = json!({"from": "2020-W53", "to": "2021-W01"});
    assert_eq!(freshness(&fixture, state.clone(), "UTC", None), Ok(()));

    build(&mut state)["coverage"] = json!({"from": "2021-W01", "to": "2021-W01"});
    let result = freshness(&fixture, state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::Coverage));
}

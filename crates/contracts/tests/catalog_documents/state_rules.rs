//! CAT-8: semantic rules of the state document.

use menura_contracts::types::catalog::RelationData;
use menura_contracts::{AdmissionCode, admit_catalog_state};
use serde_json::json;

use super::{
    admit_schema, admitted_schema, assert_state_admitted, pointer, schema_example, state_example,
    state_failure, with,
};

#[test]
fn cat_8_1_a_repeated_source_in_the_state_is_wcat_03() {
    let document = with(
        state_example(),
        "/relations/1/source",
        json!("sales.public.fact_invoice"),
    );
    assert_eq!(
        state_failure(&document),
        (AdmissionCode::Wcat03, pointer("/relations/1/source"))
    );
}

#[test]
fn cat_8_4_a_relation_source_missing_from_the_schema_is_wcat_03() {
    let document = with(
        state_example(),
        "/relations/1/source",
        json!("sales.public.dim_store"),
    );
    assert_eq!(
        state_failure(&document),
        (AdmissionCode::Wcat03, pointer("/relations/1/source"))
    );
}

#[test]
fn cat_8_4_a_check_source_missing_from_the_schema_is_wcat_03() {
    let document = with(
        state_example(),
        "/checks/0/verified/2/source",
        json!("sales.public.dim_store"),
    );
    assert_eq!(
        state_failure(&document),
        (
            AdmissionCode::Wcat03,
            pointer("/checks/0/verified/2/source")
        )
    );
}

#[test]
fn cat_8_5_a_partition_column_missing_from_the_relation_is_wcat_03() {
    let document = with(
        state_example(),
        "/relations/0/partitions/column",
        json!("created_at"),
    );
    assert_eq!(
        state_failure(&document),
        (
            AdmissionCode::Wcat03,
            pointer("/relations/0/partitions/column")
        )
    );
}

#[test]
fn cat_8_5_a_partition_column_that_is_not_a_date_is_wcat_03() {
    let document = with(
        state_example(),
        "/relations/0/partitions/column",
        json!("status"),
    );
    assert_eq!(
        state_failure(&document),
        (
            AdmissionCode::Wcat03,
            pointer("/relations/0/partitions/column")
        )
    );
}

#[test]
fn cat_8_5_a_timestamp_partition_column_is_admitted() {
    let schema = with(
        schema_example(),
        "/relations/0/columns/1/logical_type",
        json!({"kind": "timestamp", "precision": "us", "with_time_zone": true}),
    );
    let schema = admit_schema(&schema).expect("schema is admitted");
    assert!(admit_catalog_state(&state_example().to_string(), &schema).is_ok());
}

#[test]
fn cat_8_6_a_period_of_another_grain_is_wcat_03() {
    let document = with(
        state_example(),
        "/relations/0/partitions/versions/1/period",
        json!("2026-10-01"),
    );
    assert_eq!(
        state_failure(&document),
        (
            AdmissionCode::Wcat03,
            pointer("/relations/0/partitions/versions/1/period")
        )
    );
}

#[test]
fn cat_8_6_a_date_that_does_not_exist_is_wcat_03() {
    let document = with(
        state_example(),
        "/relations/0/partitions/grain",
        json!("day"),
    );
    let document = with(
        document,
        "/relations/0/partitions/versions",
        json!([{"period": "2026-02-29", "version": {"origin": "o", "snapshot": "1"}}]),
    );
    assert_eq!(
        state_failure(&document),
        (
            AdmissionCode::Wcat03,
            pointer("/relations/0/partitions/versions/0/period")
        )
    );
}

#[test]
fn cat_8_6_week_53_exists_only_in_long_iso_years() {
    let weeks = |period: &str| {
        let document = with(
            state_example(),
            "/relations/0/partitions/grain",
            json!("week"),
        );
        with(
            document,
            "/relations/0/partitions/versions",
            json!([{"period": period, "version": {"origin": "o", "snapshot": "1"}}]),
        )
    };
    // 2021-01-03 is in week 2020-W53 (CAT-5.1); 2026 has 53 weeks too, 2025 does not.
    assert_state_admitted(&weeks("2020-W53"));
    assert_state_admitted(&weeks("2026-W53"));
    assert_eq!(
        state_failure(&weeks("2025-W53")),
        (
            AdmissionCode::Wcat03,
            pointer("/relations/0/partitions/versions/0/period")
        )
    );
}

#[test]
fn cat_8_6_a_repeated_partition_period_is_wcat_03() {
    let document = with(
        state_example(),
        "/relations/0/partitions/versions/1/period",
        json!("2026-09"),
    );
    assert_eq!(
        state_failure(&document),
        (
            AdmissionCode::Wcat03,
            pointer("/relations/0/partitions/versions/1/period")
        )
    );
}

#[test]
fn cat_8_6_a_repeated_built_from_period_is_wcat_03() {
    let document = with(
        state_example(),
        "/relations/2/aggregate_build/built_from/1/period",
        json!("2026-09"),
    );
    assert_eq!(
        state_failure(&document),
        (
            AdmissionCode::Wcat03,
            pointer("/relations/2/aggregate_build/built_from/1/period")
        )
    );
}

#[test]
fn cat_8_6_summary_periods_of_mixed_grains_are_wcat_03() {
    let document = with(
        state_example(),
        "/relations/2/aggregate_build/integrity_check/verified/0/period",
        json!("2026-Q3"),
    );
    assert_eq!(
        state_failure(&document),
        (
            AdmissionCode::Wcat03,
            pointer("/relations/2/aggregate_build/integrity_check/verified/0/period")
        )
    );
}

#[test]
fn cat_8_6_a_repeated_check_version_is_wcat_03() {
    let document = with(
        state_example(),
        "/checks/0/verified/1/period",
        json!("2026-09"),
    );
    assert_eq!(
        state_failure(&document),
        (
            AdmissionCode::Wcat03,
            pointer("/checks/0/verified/1/period")
        )
    );
}

#[test]
fn cat_8_7_a_coverage_that_ends_before_it_starts_is_wcat_03() {
    let document = with(
        state_example(),
        "/relations/2/aggregate_build/coverage/from",
        json!("2026-11"),
    );
    assert_eq!(
        state_failure(&document),
        (
            AdmissionCode::Wcat03,
            pointer("/relations/2/aggregate_build/coverage/to")
        )
    );
}

#[test]
fn cat_8_7_a_coverage_of_one_period_is_admitted() {
    let document = with(
        state_example(),
        "/relations/2/aggregate_build/coverage/to",
        json!("2026-09"),
    );
    assert_state_admitted(&document);
}

#[test]
fn cat_8_9_periods_without_rows_are_not_listed() {
    // A gap between listed periods, and a partitioned relation without any row, are admitted.
    let gap = with(
        state_example(),
        "/relations/0/partitions/versions/1/period",
        json!("2026-12"),
    );
    assert_state_admitted(&gap);
    let empty = with(
        state_example(),
        "/relations/0/partitions/versions",
        json!([]),
    );
    let state = admit_catalog_state(&empty.to_string(), &admitted_schema()).expect("admitted");
    let RelationData::Partitions(partitions) = &state.relations[0].data else {
        panic!("the relation is partitioned");
    };
    assert!(partitions.versions.is_empty());
}

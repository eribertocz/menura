//! Unit tests of the `ClickHouse` integrity-check queries (E6-7.11) and of the types the
//! dialect cannot express (E6-7.12), each with its exact SQL text or error.

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::LogicalType;
use menura_contracts::types::compiled_plan::InputDocument;
use menura_contracts::types::descriptions::{Binding, CheckParameter};
use menura_generator_clickhouse::{GenerateError, generate_check_sql};
use menura_representations::origin::Origin;
use support::{
    SETTINGS, catalog_for, column, from_model, from_query, generate, instant, model, named,
    project, query, scan, scan_of,
};

#[test]
fn e6_7_11_an_integrity_check_query_follows_the_same_rules() {
    let plan = project(
        scan("ORDERS", &["CREATED_AT"]),
        vec![named("DAY", "DATE_TRUNC('day', CREATED_AT)")],
        vec![column("DAY", instant(), true)],
        from_model("/semantic_model/0/checks/0"),
    );
    let model = model();
    let generated = generate_check_sql(&plan, &catalog_for(&model), &model).expect("SQL");
    let expected = format!(
        r#"WITH
  "menura_1_scan" AS (
    SELECT
      "created_at" AS "CREATED_AT"
    FROM "main"."orders"
  ),
  "menura_2_project" AS (
    SELECT
      CAST(toTimeZone(toDateTime64(toDate32(toTimeZone("CREATED_AT", {{p1:String}})), 6, {{p1:String}}), 'UTC') AS Nullable(DateTime64(6, 'UTC'))) AS "DAY"
    FROM "menura_1_scan"
  )
SELECT
  "DAY"
FROM "menura_2_project"
{SETTINGS}"#
    );
    assert_eq!(generated.sql, expected);
    assert!(matches!(
        generated.parameters.as_slice(),
        [CheckParameter::Bound(slot)] if slot.position == 1 && slot.binding == Binding::TimeZone
    ));
}

#[test]
fn e6_7_12_a_timestamp_without_time_zone_is_e6_01_at_its_field() {
    let plan = project(
        scan("ORDERS", &["REGION", "SHIPPED_AT"]),
        vec![named("REGION", "REGION")],
        vec![column("REGION", LogicalType::Text, true)],
        from_query("/query/dimensions/0"),
    );
    let error = generate(&plan, &query(&["REGION"], &[])).expect_err("E6-01");
    assert_eq!(
        error,
        GenerateError::InexpressibleType {
            origin: from_model("/semantic_model/0/datasets/0"),
        }
    );
    let diagnostic = error.to_diagnostic().expect("a diagnostic");
    assert_eq!(diagnostic.code, "E6-01");
    let location = diagnostic.location.expect("a location");
    assert_eq!(location.document, InputDocument::Model);
    assert_eq!(location.pointer, "/semantic_model/0/datasets/0");
}

#[test]
fn e6_7_12_a_returned_time_is_e6_01_at_the_node_that_returns_it() {
    let plan = project(
        scan_of("ORDERS", vec![column("REGION", LogicalType::Text, true)]),
        vec![named("T", "REGION")],
        vec![column("T", LogicalType::Time, true)],
        from_query("/query/dimensions/0"),
    );
    let error = generate(&plan, &query(&["T"], &[])).expect_err("E6-01");
    let Origin::Query(location) = error.origin() else {
        panic!("a query origin: {error:?}");
    };
    assert_eq!(location.pointer(), "/query/dimensions/0");
    assert!(error.violation().is_none());
}

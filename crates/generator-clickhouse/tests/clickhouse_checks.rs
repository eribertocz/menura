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
use menura_model::internal_model::{FieldPath, NormalizedName};
use menura_representations::bound_query::{
    BoundDimension, BoundDimensionKind, BoundFilter, BoundQuery, BoundQueryParts,
};
use menura_representations::columns::Columns;
use menura_representations::logical_plan::PlanNode;
use menura_representations::names::OutputName;
use menura_representations::origin::Origin;
use menura_representations::time::{QueryTime, TimeZoneName};
use support::{
    SETTINGS, at, catalog_for, column, from_model, from_query, generate, generate_snapshot,
    instant, model, model_with_sources, named, parse, project, query, scan, scan_of,
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
      CAST(toTimeZone(toDateTime64(toDate32("CREATED_AT", {{p1:String}}), 6, {{p1:String}}), 'UTC') AS Nullable(DateTime64(6, 'UTC'))) AS "DAY"
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

/// The query of the dimensions `names`, the dimension at index n located at
/// `/query/dimensions/n`, with `filters`, each `(text, pointer)` over `ORDERS`.
#[cfg(test)]
fn located_query(names: &[&str], filters: &[(&str, &str)]) -> BoundQuery {
    let dimensions = names
        .iter()
        .enumerate()
        .map(|(index, name)| BoundDimension {
            kind: BoundDimensionKind::Field {
                field: FieldPath {
                    dataset: NormalizedName::new("ORDERS"),
                    field: NormalizedName::new(name),
                },
                time_grain: None,
                time_grain_origin: None,
            },
            output_name: OutputName::new(name),
            origin: at(&format!("/query/dimensions/{index}")),
        })
        .collect();
    let parts = BoundQueryParts {
        filters: filters
            .iter()
            .map(|(text, pointer)| BoundFilter {
                expression: parse(text),
                datasets: vec![NormalizedName::new("ORDERS")],
                origin: at(pointer),
            })
            .collect(),
        ..BoundQueryParts::default()
    };
    let time = QueryTime {
        time_zone: TimeZoneName::new("Europe/Madrid"),
        reference_time: None,
        reference_date: None,
    };
    BoundQuery::new(
        Columns::dimensions_then_metrics(dimensions, Vec::new()),
        time,
        parts,
    )
}

/// The `Project` of the `REGION` of `ORDERS`, the plan the queries below would have.
fn region_plan() -> PlanNode {
    project(
        scan("ORDERS", &["REGION"]),
        vec![named("REGION", "REGION")],
        vec![column("REGION", LogicalType::Text, true)],
        from_query("/query/dimensions/0"),
    )
}

#[test]
fn e6_7_12_a_field_without_time_zone_is_e6_01_at_its_dimension() {
    let query = located_query(&["REGION", "SHIPPED_AT"], &[]);
    let error = generate(&region_plan(), &query).expect_err("E6-01");
    assert_eq!(
        error,
        GenerateError::InexpressibleType {
            origin: from_query("/query/dimensions/1"),
            subject: Some("field:ORDERS.SHIPPED_AT".to_owned()),
        }
    );
    let diagnostic = error.to_diagnostic().expect("a diagnostic");
    assert_eq!(diagnostic.code, "E6-01");
    assert!(
        diagnostic
            .message
            .starts_with("`field:ORDERS.SHIPPED_AT` needs a value")
    );
    let location = diagnostic.location.expect("a location");
    assert_eq!(location.document, InputDocument::Query);
    assert_eq!(location.pointer, "/query/dimensions/1");
}

#[test]
fn e6_7_12_a_value_inside_an_expression_is_e6_01_at_its_element() {
    let query = located_query(
        &["REGION"],
        &[
            ("REGION = 'north'", "/query/filters/0"),
            ("CREATED_AT < CURRENT_TIMESTAMP", "/query/filters/1"),
        ],
    );
    let error = generate(&region_plan(), &query).expect_err("E6-01");
    assert_eq!(
        error,
        GenerateError::InexpressibleType {
            origin: from_query("/query/filters/1"),
            subject: None,
        }
    );
}

#[test]
fn e6_7_12_a_date_parameter_outside_date32_is_e6_01() {
    let query = located_query(
        &["REGION"],
        &[("ORDERED_ON > DATE '1899-12-31'", "/query/filters/0")],
    );
    let error = generate(&region_plan(), &query).expect_err("E6-01");
    assert_eq!(error.origin(), &from_query("/query/filters/0"));
    let query = located_query(
        &["REGION"],
        &[("ORDERED_ON > DATE '1900-01-01'", "/query/filters/0")],
    );
    assert!(generate(&region_plan(), &query).is_ok());
}

#[test]
fn e6_7_12_a_value_only_the_model_reads_is_e6_01_at_its_dataset() {
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
            subject: Some("dataset:ORDERS".to_owned()),
        }
    );
    let location = error
        .to_diagnostic()
        .and_then(|diagnostic| diagnostic.location);
    assert_eq!(
        location.map(|location| location.document),
        Some(InputDocument::Model)
    );
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

#[test]
fn e6_7_12_a_value_the_population_of_a_snapshot_check_reads_is_e6_01_at_its_dataset() {
    let model = model_with_sources("main.orders", "main.staged");
    let rows = scan("ORDERS", &["REGION", "CREATED_AT"]);
    let population = scan("STAGED", &["REGION", "CREATED_AT", "SHIPPED_AT"]);
    let error = generate_snapshot(&rows, &population, &model).expect_err("E6-01");
    assert_eq!(
        error,
        GenerateError::InexpressibleType {
            origin: from_model("/semantic_model/0/datasets/0"),
            subject: Some("dataset:STAGED".to_owned()),
        }
    );
}

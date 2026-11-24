//! Unit tests of the relation sources the `ClickHouse` dialect can express (E6-7.1): one part,
//! `table`, or two parts, `database.table`; three or more parts stop with `E6-01`.

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::LogicalType;
use menura_contracts::types::compiled_plan::InputDocument;
use menura_generator_clickhouse::{GenerateError, GeneratedSql, generate_check_sql};
use menura_model::internal_model::InternalModel;
use menura_representations::logical_plan::PlanNode;
use support::{
    catalog_for, column, from_model, from_query, generate_sql, home_scan, joined_scan, left_join,
    model_with_sources, named, project, query, scan, security,
};

/// The `Project` of the `REGION` of `dataset`.
fn region_plan(dataset: &str) -> PlanNode {
    project(
        scan(dataset, &["REGION"]),
        vec![named("REGION", "REGION")],
        vec![column("REGION", LogicalType::Text, true)],
        from_query("/query/dimensions/0"),
    )
}

/// The SQL of `plan` for the query of the dimensions `names` over `model`.
fn generate_over(
    plan: &PlanNode,
    model: &InternalModel,
    names: &[&str],
) -> Result<GeneratedSql, GenerateError> {
    generate_sql(plan, model, &query(names, &[]), &security())
}

/// The `E6-01` of a relation source of too many parts at `dataset:<dataset>`.
fn source_error(dataset: &str) -> GenerateError {
    GenerateError::InexpressibleSource {
        origin: from_model("/semantic_model/0/datasets/0"),
        dataset: dataset.to_owned(),
    }
}

const QUERY_SOURCE: &str = "select * from \"raw\".\"orders\"";

#[test]
fn e6_7_1_a_source_of_three_parts_is_e6_01_at_its_dataset() {
    let model = model_with_sources("warehouse.main.orders", QUERY_SOURCE);
    let error = generate_over(&region_plan("ORDERS"), &model, &["REGION"]).expect_err("E6-01");
    assert_eq!(error, source_error("ORDERS"));
    assert!(error.violation().is_none());
    let diagnostic = error.to_diagnostic().expect("a diagnostic");
    assert_eq!(diagnostic.code, "E6-01");
    assert!(diagnostic.message.starts_with("`dataset:ORDERS` "));
    let location = diagnostic.location.expect("a location");
    assert_eq!(location.document, InputDocument::Model);
    assert_eq!(location.pointer, "/semantic_model/0/datasets/0");
}

#[test]
fn e6_7_1_a_source_of_four_parts_is_e6_01() {
    let model = model_with_sources("a.b.c.d", QUERY_SOURCE);
    let error = generate_over(&region_plan("ORDERS"), &model, &["REGION"]).expect_err("E6-01");
    assert_eq!(error, source_error("ORDERS"));
}

#[test]
fn e6_7_1_the_first_dataset_in_scan_order_is_reported() {
    let model = model_with_sources("a.main.orders", "b.main.staged");
    let staged_first = left_join(
        home_scan("STAGED", &["REGION"]),
        joined_scan("ORDERS", "TO_ORDERS.", &["REGION"]),
        "TO_ORDERS",
        &[("REGION", "TO_ORDERS.REGION")],
    );
    let names = ["REGION", "TO_ORDERS.REGION"];
    let error = generate_over(&staged_first, &model, &names).expect_err("E6-01");
    assert!(
        matches!(error, GenerateError::InexpressibleSource { dataset, .. } if dataset == "STAGED")
    );
    let orders_first = left_join(
        home_scan("ORDERS", &["REGION"]),
        joined_scan("STAGED", "TO_STAGED.", &["REGION"]),
        "TO_STAGED",
        &[("REGION", "TO_STAGED.REGION")],
    );
    let names = ["REGION", "TO_STAGED.REGION"];
    let error = generate_over(&orders_first, &model, &names).expect_err("E6-01");
    assert!(
        matches!(error, GenerateError::InexpressibleSource { dataset, .. } if dataset == "ORDERS")
    );
}

#[test]
fn e6_7_1_the_source_check_runs_before_the_type_check() {
    let model = model_with_sources("a.main.orders", QUERY_SOURCE);
    // `SHIPPED_AT` is a `timestamp` without time zone, which E6-7.12 rejects.
    let error = generate_over(&region_plan("ORDERS"), &model, &["REGION", "SHIPPED_AT"])
        .expect_err("E6-01");
    assert_eq!(error, source_error("ORDERS"));
}

#[test]
fn e6_7_1_sources_of_one_or_two_parts_and_query_sources_are_unaffected() {
    for source in [
        "orders",
        "main.orders",
        "\"my.db\".orders",
        "\"a.b.c\"",
        " main . \"Order.Lines\" ",
        QUERY_SOURCE,
        "  WITH x AS (SELECT 1) SELECT * FROM a.b.c",
    ] {
        let model = model_with_sources(source, QUERY_SOURCE);
        let generated = generate_over(&region_plan("ORDERS"), &model, &["REGION"]);
        assert!(generated.is_ok(), "{source}: {generated:?}");
    }
}

#[test]
fn e6_7_1_an_integrity_check_applies_the_same_check() {
    let model = model_with_sources("a.main.orders", QUERY_SOURCE);
    let plan = project(
        scan("ORDERS", &["REGION"]),
        vec![named("REGION", "REGION")],
        vec![column("REGION", LogicalType::Text, true)],
        from_model("/semantic_model/0/checks/0"),
    );
    let error = generate_check_sql(&plan, &catalog_for(&model), &model).expect_err("E6-01");
    assert_eq!(error, source_error("ORDERS"));
    let model = model_with_sources("main.orders", QUERY_SOURCE);
    assert!(generate_check_sql(&plan, &catalog_for(&model), &model).is_ok());
}

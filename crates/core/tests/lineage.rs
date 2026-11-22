//! The lineage of compiled plans (PLN-4).

mod support;

use menura_contracts::types::compiled_plan::Lineage;
use menura_core::{Handle, LoadOptions};
use support::{compile_default, context, handle_with, plan, query};

const MODEL: &str = include_str!("../../../conformance/models/pln-lineage/model.yaml");
const CATALOG_SCHEMA: &str =
    include_str!("../../../conformance/models/pln-lineage/catalog-schema.json");
const SUMMARY_MODEL: &str =
    include_str!("../../../conformance/models/e5-summary-selection/model.yaml");
const SUMMARY_CATALOG_SCHEMA: &str =
    include_str!("../../../conformance/models/e5-summary-selection/catalog-schema.json");
const SUMMARY_CATALOG_STATE: &str =
    include_str!("../../../conformance/models/e5-summary-selection/catalog-state.json");

fn lineage_of(handle: &Handle, query_member: &str) -> Vec<Lineage> {
    let analyst = context("analyst-1", &["analyst"], "{}");
    plan(compile_default(handle, &query(query_member), &analyst)).lineage
}

fn lineage_handle() -> Handle {
    handle_with(MODEL, CATALOG_SCHEMA, None, &LoadOptions::default())
}

fn summary_handle() -> Handle {
    handle_with(
        SUMMARY_MODEL,
        SUMMARY_CATALOG_SCHEMA,
        Some(SUMMARY_CATALOG_STATE),
        &LoadOptions::default(),
    )
}

/// `total_amount` by `stores.region`: one branch, home `orders`, joined to `stores`.
const BY_REGION: &str = r#"{"dimensions": ["stores.region"], "metrics": ["total_amount"]}"#;

/// Three metrics by `customers.segment`: the `orders` branch, the `customers` branch and a
/// derived metric over both.
const BY_SEGMENT: &str = r#"{"dimensions": ["customers.segment"],
    "metrics": ["total_amount", "customer_count", "amount_per_customer"]}"#;

/// `physical_inputs` written `source.column`.
fn physical(entry: &Lineage) -> Vec<String> {
    entry
        .physical_inputs
        .iter()
        .map(|input| format!("{}.{}", input.source, input.column))
        .collect()
}

#[test]
fn pln_4_3_one_entry_per_result_column_in_order() {
    let handle = lineage_handle();
    let analyst = context("analyst-1", &["analyst"], "{}");
    let compiled = plan(compile_default(&handle, &query(BY_SEGMENT), &analyst));
    let outputs: Vec<&str> = compiled
        .lineage
        .iter()
        .map(|entry| entry.output.as_str())
        .collect();
    let columns: Vec<&str> = compiled
        .result_schema
        .iter()
        .map(|column| column.name.as_str())
        .collect();
    assert_eq!(outputs, columns);
    assert_eq!(
        outputs,
        [
            "segment",
            "total_amount",
            "customer_count",
            "amount_per_customer"
        ]
    );
}

#[test]
fn pln_4_4_a_dimension_derives_from_its_field_and_the_join_that_brings_it() {
    let lineage = lineage_of(&lineage_handle(), BY_REGION);
    assert_eq!(
        lineage[0].model_inputs,
        ["field:STORES.REGION", "relationship:ORDERS_STORES"]
    );
}

#[test]
fn pln_4_4_a_metric_derives_from_itself_and_its_fields() {
    let lineage = lineage_of(&lineage_handle(), BY_REGION);
    assert_eq!(
        lineage[1].model_inputs,
        ["field:ORDERS.AMOUNT", "metric:TOTAL_AMOUNT"]
    );
}

#[test]
fn pln_4_4_a_derived_metric_derives_from_its_inputs_transitively() {
    let lineage = lineage_of(&lineage_handle(), BY_SEGMENT);
    assert_eq!(
        lineage[3].model_inputs,
        [
            "field:CUSTOMERS.CUSTOMER_ID",
            "field:ORDERS.AMOUNT",
            "metric:AMOUNT_PER_CUSTOMER",
            "metric:CUSTOMER_COUNT",
            "metric:TOTAL_AMOUNT",
        ]
    );
}

#[test]
fn pln_4_5_physical_inputs_include_join_and_policy_reads() {
    let lineage = lineage_of(&lineage_handle(), BY_REGION);
    assert_eq!(
        physical(&lineage[1]),
        [
            "shop.main.orders.amount",
            "shop.main.orders.store_id",
            "shop.main.stores.region",
            "shop.main.stores.store_id",
        ]
    );
}

#[test]
fn pln_4_5_a_summary_branch_reads_the_summary_relation() {
    let lineage = lineage_of(
        &summary_handle(),
        r#"{"dimensions": ["orders.region"], "metrics": ["total_amount"]}"#,
    );
    assert_eq!(
        physical(&lineage[1]),
        ["shop.main.orders_by_region_month.amount_sum"]
    );
    assert_eq!(
        lineage[1].model_inputs,
        ["field:ORDERS.AMOUNT", "metric:TOTAL_AMOUNT"]
    );
}

#[test]
fn pln_4_5_a_recomposed_average_reads_the_summary_count_column() {
    let lineage = lineage_of(
        &summary_handle(),
        r#"{"dimensions": ["orders.region"], "metrics": ["average_amount"]}"#,
    );
    assert_eq!(
        physical(&lineage[1]),
        [
            "shop.main.orders_by_region_month.amount_count",
            "shop.main.orders_by_region_month.amount_sum",
        ]
    );
}

#[test]
fn pln_4_6_summary_names_the_summary_used() {
    let summary = lineage_of(
        &summary_handle(),
        r#"{"dimensions": ["orders.region"], "metrics": ["total_amount"]}"#,
    );
    assert!(
        summary
            .iter()
            .all(|entry| entry.summary.as_deref() == Some("ORDERS_BY_REGION_MONTH"))
    );
    let base = lineage_of(&lineage_handle(), BY_REGION);
    assert!(base.iter().all(|entry| entry.summary.is_none()));
}

#[test]
fn pln_4_7_propagated_policies_are_listed() {
    let lineage = lineage_of(&lineage_handle(), BY_REGION);
    assert!(
        lineage
            .iter()
            .all(|entry| entry.security_policies == ["STORES.NORTH_ONLY"])
    );
}

#[test]
fn pln_4_7_only_applicable_policies_are_listed() {
    let north = context("summaries-north", &["north"], "{}");
    let by_region = query(r#"{"dimensions": ["orders.region"], "metrics": ["total_amount"]}"#);
    let compiled = plan(compile_default(&summary_handle(), &by_region, &north));
    assert!(
        compiled
            .lineage
            .iter()
            .all(|entry| entry.security_policies == ["ORDERS.NORTH_ONLY"])
    );
}

#[test]
fn pln_4_8_a_metric_reads_only_the_branches_of_its_components() {
    let lineage = lineage_of(&lineage_handle(), BY_SEGMENT);
    assert_eq!(physical(&lineage[2]), ["shop.main.customers.customer_id"]);
    assert!(lineage[2].security_policies.is_empty());
    assert_eq!(lineage[3].security_policies, ["STORES.NORTH_ONLY"]);
}

#[test]
fn pln_4_8_a_dimension_reads_every_branch() {
    let lineage = lineage_of(&lineage_handle(), BY_SEGMENT);
    assert_eq!(
        physical(&lineage[0]),
        [
            "shop.main.customers.customer_id",
            "shop.main.customers.segment",
            "shop.main.orders.customer_id",
            "shop.main.orders.store_id",
            "shop.main.stores.region",
            "shop.main.stores.store_id",
        ]
    );
    assert_eq!(lineage[0].security_policies, ["STORES.NORTH_ONLY"]);
}

#[test]
fn pln_4_2_lineage_holds_no_query_literal() {
    let handle = lineage_handle();
    let analyst = context("analyst-1", &["analyst"], "{}");
    let filtered = query(
        r#"{"dimensions": ["stores.region"], "metrics": ["total_amount"],
            "nonportable_extensions": [{"vendor_name": "COMMON", "data": {"namespace": "menura",
            "version": 1, "filters": ["customers.segment = 'SECRET_SEGMENT'"]}}]}"#,
    );
    let compiled = plan(compile_default(&handle, &filtered, &analyst));
    let text = serde_json::to_string(&compiled.lineage).unwrap();
    assert!(!text.contains("SECRET_SEGMENT"));
    assert!(text.contains("\"column\":\"segment\""));
}

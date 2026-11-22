//! `describe` (DSC): the model description of the `dsc-describe` conformance model for an
//! admin, a restricted analyst and a context that is not admitted.

#![cfg(test)]

use menura_core::{
    AiContext, DescribeResult, Handle, LoadOptions, MetricKind, ModelDescriptionDocument, describe,
    load,
};
use serde_json::json;

const MODEL: &str = include_str!("../../../conformance/models/dsc-describe/model.yaml");
const CATALOG: &str = include_str!("../../../conformance/models/dsc-describe/catalog-schema.json");
const ADMIN: &str = include_str!("../../../conformance/models/dsc-describe/contexts/admin.json");
const ANALYST: &str =
    include_str!("../../../conformance/models/dsc-describe/contexts/analyst.json");

fn handle() -> Handle {
    load(
        MODEL.as_bytes(),
        CATALOG.as_bytes(),
        None,
        &LoadOptions::default(),
    )
    .unwrap()
}

fn description(context: &str) -> ModelDescriptionDocument {
    match describe(&handle(), context.as_bytes()) {
        DescribeResult::Description(document) => *document,
        DescribeResult::Error(error) => panic!("expected a description, got {error:?}"),
    }
}

fn dataset_names(document: &ModelDescriptionDocument) -> Vec<&str> {
    document
        .datasets
        .iter()
        .map(|dataset| dataset.name.as_str())
        .collect()
}

fn field_references<'document>(
    document: &'document ModelDescriptionDocument,
    dataset: &str,
) -> Vec<&'document str> {
    document
        .datasets
        .iter()
        .find(|described| described.name == dataset)
        .unwrap()
        .fields
        .iter()
        .map(|field| field.reference.as_str())
        .collect()
}

fn metric_references(document: &ModelDescriptionDocument) -> Vec<&str> {
    document
        .metrics
        .iter()
        .map(|metric| metric.reference.as_str())
        .collect()
}

fn json_of(document: &ModelDescriptionDocument) -> serde_json::Value {
    serde_json::to_value(document).unwrap()
}

#[test]
fn dsc_3_1_dsc_3_2_only_authorized_datasets_are_listed_and_summaries_never() {
    assert_eq!(
        dataset_names(&description(ADMIN)),
        ["CUSTOMERS", "ORDERS", "STORES", "SUPPLIERS"]
    );
    assert_eq!(
        dataset_names(&description(ANALYST)),
        ["CUSTOMERS", "ORDERS", "STORES"]
    );
}

#[test]
fn dsc_3_1_dsc_3_7_restricted_unusable_and_unsupported_fields_are_omitted() {
    assert_eq!(
        field_references(&description(ADMIN), "ORDERS"),
        [
            "ORDERS.AMOUNT",
            "ORDERS.CUSTOMER_ID",
            "ORDERS.MARGIN",
            "ORDERS.ORDER_DAY",
            "ORDERS.ORDER_ID",
            "ORDERS.PICKUP_STORE_ID",
            "ORDERS.SHIPPED_AT",
            "ORDERS.STORE_ID",
        ]
    );
    assert_eq!(
        field_references(&description(ANALYST), "ORDERS"),
        [
            "ORDERS.AMOUNT",
            "ORDERS.CUSTOMER_ID",
            "ORDERS.ORDER_DAY",
            "ORDERS.ORDER_ID",
            "ORDERS.SHIPPED_AT",
            "ORDERS.STORE_ID",
        ]
    );
}

#[test]
fn dsc_3_1_e2_4_3_metrics_are_listed_only_when_authorized_and_usable() {
    assert_eq!(
        metric_references(&description(ADMIN)),
        [
            "STORE_COUNT",
            "SUPPLIER_COUNT",
            "TOTAL_AMOUNT",
            "TOTAL_MARGIN"
        ]
    );
    assert_eq!(
        metric_references(&description(ANALYST)),
        ["STORE_COUNT", "TOTAL_AMOUNT"]
    );
}

#[test]
fn dsc_2_e3_3_4_a_metric_lists_the_datasets_every_home_dataset_reaches() {
    let document = description(ANALYST);
    let total = &document.metrics[1];
    assert_eq!(total.kind, MetricKind::Base);
    assert_eq!(total.reachable_datasets, ["CUSTOMERS", "ORDERS", "STORES"]);
    assert_eq!(document.metrics[0].reachable_datasets, ["STORES"]);
}

#[test]
fn dsc_3_8_time_grains_follow_the_logical_type_and_is_time_the_declaration() {
    let document = json_of(&description(ANALYST));
    let fields = &document["datasets"][1]["fields"];
    let grains = json!(["day", "week", "month", "quarter", "year"]);
    assert_eq!(fields[2]["reference"], "ORDERS.ORDER_DAY");
    assert_eq!(fields[2]["is_time"], true);
    assert_eq!(fields[2]["time_grains"], grains);
    assert_eq!(fields[4]["reference"], "ORDERS.SHIPPED_AT");
    assert_eq!(fields[4]["is_time"], false);
    assert_eq!(fields[4]["time_grains"], grains);
    assert_eq!(fields[0]["time_grains"], json!([]));
}

#[test]
fn dsc_3_9_dsc_3_10_witness_paths_are_the_two_smallest_over_the_authorized_subgraph() {
    let admin = json_of(&description(ADMIN));
    assert_eq!(
        admin["ambiguous_paths"],
        json!([{"from": "ORDERS", "to": "STORES",
                "paths": [["ORDERS_PICKUP_STORES"], ["ORDERS_STORES"]]}])
    );
    let analyst = json_of(&description(ANALYST));
    assert_eq!(
        analyst["ambiguous_paths"],
        json!([{"from": "ORDERS", "to": "STORES",
                "paths": [["ORDERS_STORES"], ["ORDERS_CUSTOMERS", "CUSTOMERS_STORES"]]}])
    );
}

#[test]
fn dsc_3_6_dsc_3_11_ai_context_carries_its_provenance_and_optional_members_are_absent() {
    let document = description(ANALYST);
    let hash = document.model_hash.as_str().to_owned();
    let value = json_of(&document);
    let orders = &value["datasets"][1];
    assert_eq!(orders["description"], "One row per order");
    assert!(orders.get("label").is_none());
    assert_eq!(orders["fields"][2]["label"], "Order day");
    assert_eq!(
        orders["fields"][2]["description"],
        "The day the order was placed"
    );
    assert_eq!(
        orders["ai_context"],
        json!({"value": "One row per order; amount is the order total.",
               "source": {"pointer": "/semantic_model/0/datasets/0/ai_context",
                          "model_hash": hash},
               "trust": "untrusted_model_content"})
    );
    assert_eq!(
        orders["fields"][0]["ai_context"]["value"],
        json!({"rank": 1, "synonyms": ["revenue", "order total"]})
    );
    assert_eq!(
        orders["fields"][0]["ai_context"]["source"]["pointer"],
        "/semantic_model/0/datasets/0/fields/6/ai_context"
    );
    let customers = &value["datasets"][0];
    assert!(customers.get("ai_context").is_none());
    assert!(customers.get("description").is_none());
    assert!(customers["fields"][0].get("ai_context").is_none());
    let Some(AiContext::Attributed(metric)) = &document.metrics[1].ai_context else {
        panic!("the metric declares ai_context");
    };
    assert_eq!(
        metric.source.pointer,
        "/semantic_model/0/metrics/0/ai_context"
    );
}

#[test]
fn dsc_2_dsc_3_5_the_description_identifies_the_model_and_the_context_and_is_deterministic() {
    let first = description(ANALYST);
    assert_eq!(&first.model_hash, handle().model().hash());
    assert_ne!(
        first.security_fingerprint,
        description(ADMIN).security_fingerprint
    );
    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&description(ANALYST)).unwrap()
    );
}

#[test]
fn dsc_3_12_a_context_that_is_not_admitted_returns_the_compile_error() {
    let DescribeResult::Error(error) = describe(&handle(), b"{\"contract\": 1}") else {
        panic!("the context is not admitted");
    };
    assert_eq!(error.errors[0].code, "ESEC-03");
    let DescribeResult::Error(error) = describe(&handle(), b"\xff") else {
        panic!("the context is not UTF-8");
    };
    assert_eq!(error.errors[0].code, "ESEC-01");
}

#[test]
fn dsc_3_12_the_former_contract_name_gives_the_same_description() {
    let former = ANALYST.replace(
        "menura.security_context",
        "semantic_compiler.security_context",
    );
    assert_eq!(description(&former), description(ANALYST));
}

#[test]
fn dsc_4_1_the_capability_profile_is_always_present_and_independent_of_the_context() {
    let admin = description(ADMIN).capabilities.unwrap();
    assert_eq!(Some(admin), description(ANALYST).capabilities);
}

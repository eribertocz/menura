//! DSC-3.1 from contract version 1.3.0: each listed dataset carries `sql_metrics`, the metrics
//! a SQL-form query may aggregate on its relation (QRY-5.2), filtered and ordered like
//! `metrics`.

#![cfg(test)]

use menura_contracts::{ContractKind, validate_structure};
use menura_core::{
    DescribeResult, Handle, LoadOptions, ModelDescriptionDocument, OssieFormat, describe, load,
};

const DSC_MODEL: &str = include_str!("../../../conformance/models/dsc-describe/model.yaml");
const DSC_CATALOG: &str =
    include_str!("../../../conformance/models/dsc-describe/catalog-schema.json");
const ADMIN: &str = include_str!("../../../conformance/models/dsc-describe/contexts/admin.json");
const ANALYST: &str =
    include_str!("../../../conformance/models/dsc-describe/contexts/analyst.json");
const LINEAGE_MODEL: &str = include_str!("../../../conformance/models/pln-lineage/model.yaml");
const LINEAGE_CATALOG: &str =
    include_str!("../../../conformance/models/pln-lineage/catalog-schema.json");
const DEVELOPMENT_MODEL: &str =
    include_str!("../../../conformance/models/e0-metrics-development/model.yaml");
const DEVELOPMENT_CATALOG: &str =
    include_str!("../../../conformance/models/e0-metrics-development/catalog-schema.json");
const DEVELOPMENT_ANALYST: &str =
    include_str!("../../../conformance/models/e0-metrics-development/contexts/analyst.json");
const LINEAGE_ANALYST: &str = r#"{"contract": "menura.security_context",
    "contract_version": "1.1.0", "subject": "analyst-1", "roles": ["analyst"], "attributes": {}}"#;

fn handle(model: &str, catalog: &str, options: &LoadOptions) -> Handle {
    load(model.as_bytes(), catalog.as_bytes(), None, options).unwrap()
}

fn description(handle: &Handle, context: &str) -> ModelDescriptionDocument {
    match describe(handle, context.as_bytes()) {
        DescribeResult::Description(document) => *document,
        DescribeResult::Error(error) => panic!("expected a description, got {error:?}"),
    }
}

fn dsc_description(context: &str) -> ModelDescriptionDocument {
    let handle = handle(DSC_MODEL, DSC_CATALOG, &LoadOptions::default());
    description(&handle, context)
}

/// Each listed dataset with its `sql_metrics`, in the order of `datasets`.
fn sql_metrics(document: &ModelDescriptionDocument) -> Vec<(&str, Vec<&str>)> {
    document
        .datasets
        .iter()
        .map(|dataset| {
            let metrics = dataset
                .sql_metrics
                .as_ref()
                .unwrap()
                .iter()
                .map(String::as_str)
                .collect();
            (dataset.name.as_str(), metrics)
        })
        .collect()
}

fn metric_references(document: &ModelDescriptionDocument) -> Vec<&str> {
    document
        .metrics
        .iter()
        .map(|metric| metric.reference.as_str())
        .collect()
}

#[test]
fn dsc_3_1_sql_metrics_lists_the_metrics_exposed_on_each_relation() {
    assert_eq!(
        sql_metrics(&dsc_description(ADMIN)),
        [
            ("CUSTOMERS", vec![]),
            ("ORDERS", vec!["TOTAL_AMOUNT", "TOTAL_MARGIN"]),
            ("STORES", vec!["STORE_COUNT"]),
            ("SUPPLIERS", vec!["SUPPLIER_COUNT"]),
        ]
    );
}

#[test]
fn dsc_3_1_sql_metrics_is_filtered_exactly_as_metrics() {
    let document = dsc_description(ANALYST);
    assert_eq!(
        sql_metrics(&document),
        [
            ("CUSTOMERS", vec![]),
            ("ORDERS", vec!["TOTAL_AMOUNT"]),
            ("STORES", vec!["STORE_COUNT"]),
        ]
    );
    let listed = metric_references(&document);
    for (_, metrics) in sql_metrics(&document) {
        assert!(metrics.iter().all(|metric| listed.contains(metric)));
    }
}

#[test]
fn dsc_3_1_a_summary_and_its_metrics_are_never_in_sql_metrics() {
    let document = dsc_description(ADMIN);
    assert!(
        document
            .datasets
            .iter()
            .all(|dataset| dataset.name != "ORDERS_MONTHLY")
    );
    let orders = document
        .datasets
        .iter()
        .find(|dataset| dataset.name == "ORDERS")
        .unwrap();
    assert_eq!(
        orders.sql_metrics.as_deref(),
        Some(["TOTAL_AMOUNT".to_owned(), "TOTAL_MARGIN".to_owned()].as_slice())
    );
}

#[test]
fn dsc_3_1_a_metric_of_several_home_datasets_is_exposed_on_none_under_propagation() {
    let handle = handle(LINEAGE_MODEL, LINEAGE_CATALOG, &LoadOptions::default());
    let document = description(&handle, LINEAGE_ANALYST);
    assert!(metric_references(&document).contains(&"AMOUNT_PER_CUSTOMER"));
    assert_eq!(
        sql_metrics(&document),
        [
            ("CUSTOMERS", vec!["CUSTOMER_COUNT"]),
            ("ORDERS", vec!["TOTAL_AMOUNT"]),
            ("STORES", vec![]),
        ]
    );
}

#[test]
fn dsc_3_1_dataset_metrics_are_exposed_on_their_dataset_in_the_order_of_metrics() {
    let options = LoadOptions {
        ossie_format: OssieFormat::Development,
        ..LoadOptions::default()
    };
    let handle = handle(DEVELOPMENT_MODEL, DEVELOPMENT_CATALOG, &options);
    let document = description(&handle, DEVELOPMENT_ANALYST);
    let listed = metric_references(&document);
    assert!(!listed.is_empty());
    assert_eq!(sql_metrics(&document), [("ORDERS", listed)]);
}

#[test]
fn dsc_3_1_sql_metrics_is_always_written_and_follows_the_schema() {
    let document = dsc_description(ANALYST);
    assert_eq!(document.contract_version.to_string(), "1.3.0");
    let value = serde_json::to_value(&document).unwrap();
    assert_eq!(
        validate_structure(ContractKind::ModelDescription, &value),
        Ok(())
    );
    assert_eq!(value["datasets"][0]["sql_metrics"], serde_json::json!([]));
    let decoded: ModelDescriptionDocument = serde_json::from_value(value).unwrap();
    assert_eq!(decoded, document);
}

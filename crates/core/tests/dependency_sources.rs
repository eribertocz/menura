//! DEP-3.5 from contract version 1.1.0: each dataset entry carries `sources`, the catalog
//! relations a query on the dataset may read.

#![cfg(test)]

use menura_contracts::{ContractKind, validate_structure};
use menura_core::{LoadOptions, ModelDependenciesDocument, dependencies, load};
use serde_json::{Value, json};

const LINEAGE_MODEL: &str = include_str!("../../../conformance/models/pln-lineage/model.yaml");
const LINEAGE_SCHEMA: &str =
    include_str!("../../../conformance/models/pln-lineage/catalog-schema.json");
const PROPAGATION_MODEL: &str =
    include_str!("../../../conformance/models/multi-propagation/model.yaml");
const PROPAGATION_SCHEMA: &str =
    include_str!("../../../conformance/models/multi-propagation/catalog-schema.json");
const SUMMARY_MODEL: &str =
    include_str!("../../../conformance/models/e5-summary-selection/model.yaml");
const SUMMARY_SCHEMA: &str =
    include_str!("../../../conformance/models/e5-summary-selection/catalog-schema.json");

/// The `regions` dataset, with a row policy, and the relationship from `orders` to it, added to
/// the summary model so that the policy propagates to `orders` and its summaries.
const REGIONS_DATASET: &str = r#"      - name: regions
        source: shop.regions
        primary_key: [region]
        fields:
          - name: region
            expression: {dialects: [{dialect: ANSI_SQL, expression: 'region'}]}
        custom_extensions:
          - vendor_name: COMMON
            data: '{"namespace": "menura", "version": 1, "access": {"mode": "roles", "roles": ["analyst", "north"]}, "row_access": [{"name": "north_region", "condition": "region = ''NORTH''", "roles": ["north"]}]}'
    relationships:
      - name: orders_regions
        from: orders
        to: regions
        from_columns: [region]
        to_columns: [region]
    metrics:
"#;

fn document_of(model: &str, schema: &str) -> ModelDependenciesDocument {
    let handle = load(
        model.as_bytes(),
        schema.as_bytes(),
        None,
        &LoadOptions::default(),
    )
    .unwrap();
    dependencies(&handle).unwrap()
}

/// The summary model with `regions`, whose row policy propagates to `orders`.
fn propagated_summaries() -> ModelDependenciesDocument {
    let model = SUMMARY_MODEL.replacen("    metrics:\n", REGIONS_DATASET, 1);
    let mut schema: Value = serde_json::from_str(SUMMARY_SCHEMA).unwrap();
    schema["relations"].as_array_mut().unwrap().push(json!({
        "source": "shop.regions",
        "columns": [{
            "name": "region",
            "logical_type": {"kind": "text"},
            "physical_type": "VARCHAR",
            "nullable": false
        }]
    }));
    document_of(&model, &schema.to_string())
}

fn sources<'document>(
    document: &'document ModelDependenciesDocument,
    object: &str,
) -> Vec<&'document str> {
    document
        .datasets
        .iter()
        .find(|entry| entry.object == object)
        .unwrap_or_else(|| panic!("{object} is listed"))
        .sources
        .as_ref()
        .unwrap()
        .iter()
        .map(String::as_str)
        .collect()
}

#[test]
fn dep_3_5_a_dataset_without_policies_or_summaries_reads_its_own_source() {
    let document = document_of(LINEAGE_MODEL, LINEAGE_SCHEMA);
    assert_eq!(sources(&document, "dataset:CUSTOMERS"), ["shop.customers"]);
    assert_eq!(sources(&document, "dataset:STORES"), ["shop.stores"]);
}

#[test]
fn dep_3_5_a_propagated_policy_adds_the_source_its_semi_join_scans() {
    let document = document_of(LINEAGE_MODEL, LINEAGE_SCHEMA);
    assert_eq!(
        sources(&document, "dataset:ORDERS"),
        ["shop.orders", "shop.stores"]
    );
    let document = document_of(PROPAGATION_MODEL, PROPAGATION_SCHEMA);
    assert_eq!(
        sources(&document, "dataset:EVENTS"),
        ["shop.cities", "shop.events", "shop.venues"]
    );
    assert_eq!(
        sources(&document, "dataset:VISITS"),
        ["shop.visits", "shop.zones"]
    );
}

#[test]
fn dep_3_5_a_dataset_on_the_propagation_chain_without_policies_is_a_source() {
    let document = document_of(PROPAGATION_MODEL, PROPAGATION_SCHEMA);
    assert_eq!(
        sources(&document, "dataset:ORDERS"),
        ["shop.customers", "shop.orders", "shop.stores"]
    );
    assert_eq!(
        sources(&document, "dataset:LINES"),
        [
            "shop.lines",
            "shop.products",
            "shop.suppliers",
            "shop.warehouses"
        ]
    );
    assert_eq!(
        sources(&document, "dataset:PEOPLE"),
        ["shop.people", "shop.teams"]
    );
}

#[test]
fn dep_3_5_sources_are_computed_for_every_user_whatever_their_access() {
    let document = document_of(PROPAGATION_MODEL, PROPAGATION_SCHEMA);
    assert_eq!(
        sources(&document, "dataset:PRODUCTS"),
        ["shop.products", "shop.suppliers"]
    );
    assert_eq!(
        sources(&document, "dataset:WAREHOUSES"),
        ["shop.suppliers", "shop.warehouses"]
    );
    assert_eq!(sources(&document, "dataset:SUPPLIERS"), ["shop.suppliers"]);
}

#[test]
fn dep_3_5_a_base_reads_its_summaries_and_a_summary_the_visibility_of_its_base() {
    let document = document_of(SUMMARY_MODEL, SUMMARY_SCHEMA);
    assert_eq!(
        sources(&document, "dataset:ORDERS"),
        [
            "shop.orders",
            "shop.orders_by_month",
            "shop.orders_by_region_month"
        ]
    );
    assert_eq!(
        sources(&document, "dataset:ORDERS_BY_MONTH"),
        ["shop.orders_by_month"]
    );
    let document = propagated_summaries();
    assert_eq!(
        sources(&document, "dataset:ORDERS"),
        [
            "shop.orders",
            "shop.orders_by_month",
            "shop.orders_by_region_month",
            "shop.regions"
        ]
    );
    assert_eq!(
        sources(&document, "dataset:ORDERS_BY_MONTH"),
        ["shop.orders_by_month", "shop.regions"]
    );
    assert_eq!(sources(&document, "dataset:REGIONS"), ["shop.regions"]);
}

#[test]
fn dep_3_5_only_dataset_entries_carry_sources_and_the_document_follows_its_schema() {
    let document = propagated_summaries();
    assert_eq!(document.contract_version.to_string(), "1.1.0");
    assert!(
        document
            .datasets
            .iter()
            .all(|entry| entry.sources.is_some())
    );
    assert!(
        document
            .relationships
            .iter()
            .chain(&document.fields)
            .chain(&document.policies)
            .all(|entry| entry.sources.is_none())
    );
    let value = serde_json::to_value(&document).unwrap();
    assert_eq!(
        validate_structure(ContractKind::ModelDependencies, &value),
        Ok(())
    );
    let decoded: ModelDependenciesDocument = serde_json::from_value(value).unwrap();
    assert_eq!(decoded, document);
}

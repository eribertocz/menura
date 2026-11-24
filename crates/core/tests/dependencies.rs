//! The model-dependency document (DEP-1 to DEP-3) and the model diff (DEP-4).
//!
//! `cfg(test)` marks the helpers as test code, where `unwrap` and `panic!` are allowed
//! (clippy.toml).

mod support;

use menura_contracts::{ContractKind, validate_structure};
use menura_core::{
    ChangeKind, ElementDependencies, Handle, LoadOptions, MetricDependencies, MetricKind,
    ModelDependenciesDocument, OssieFormat, dependencies, diff,
};
use support::handle_with;

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
const SUMMARY_STATE: &str =
    include_str!("../../../conformance/models/e5-summary-selection/catalog-state.json");
const CONTEXT_MODEL: &str = include_str!("../../../conformance/models/e0-context/model.yaml");
const CONTEXT_SCHEMA: &str =
    include_str!("../../../conformance/models/e0-context/catalog-schema.json");
const SEMI_ADDITIVE_MODEL: &str =
    include_str!("../../../conformance/models/next-semi-additive/model.yaml");
const SEMI_ADDITIVE_SCHEMA: &str =
    include_str!("../../../conformance/models/next-semi-additive/catalog-schema.json");
const UNUSABLE_MODEL: &str =
    include_str!("../../../conformance/models/e0-derived-unusable/model.yaml");
const UNUSABLE_SCHEMA: &str =
    include_str!("../../../conformance/models/e0-derived-unusable/catalog-schema.json");
const DEVELOPMENT_MODEL: &str =
    include_str!("../../../conformance/models/e0-metrics-development/model.yaml");
const DEVELOPMENT_SCHEMA: &str =
    include_str!("../../../conformance/models/e0-metrics-development/catalog-schema.json");

#[cfg(test)]
fn document_of(model: &str, schema: &str) -> ModelDependenciesDocument {
    let handle = handle_with(model, schema, None, &LoadOptions::default());
    dependencies(&handle).unwrap()
}

#[cfg(test)]
fn lineage() -> ModelDependenciesDocument {
    document_of(LINEAGE_MODEL, LINEAGE_SCHEMA)
}

#[cfg(test)]
fn entry<'document>(
    entries: &'document [ElementDependencies],
    object: &str,
) -> &'document ElementDependencies {
    entries
        .iter()
        .find(|entry| entry.object == object)
        .unwrap_or_else(|| panic!("{object} is listed"))
}

#[cfg(test)]
fn metric<'document>(
    document: &'document ModelDependenciesDocument,
    object: &str,
) -> &'document MetricDependencies {
    document
        .metrics
        .iter()
        .find(|entry| entry.object == object)
        .unwrap_or_else(|| panic!("{object} is listed"))
}

#[cfg(test)]
fn physical(inputs: &[menura_contracts::types::compiled_plan::PhysicalInput]) -> Vec<String> {
    inputs
        .iter()
        .map(|input| format!("{}.{}", input.source, input.column))
        .collect()
}

#[cfg(test)]
fn objects(entries: &[ElementDependencies]) -> Vec<&str> {
    entries.iter().map(|entry| entry.object.as_str()).collect()
}

#[test]
fn dep_1_the_document_follows_its_schema_and_round_trips() {
    for document in [
        lineage(),
        document_of(PROPAGATION_MODEL, PROPAGATION_SCHEMA),
        document_of(SUMMARY_MODEL, SUMMARY_SCHEMA),
        document_of(CONTEXT_MODEL, CONTEXT_SCHEMA),
        document_of(SEMI_ADDITIVE_MODEL, SEMI_ADDITIVE_SCHEMA),
        document_of(UNUSABLE_MODEL, UNUSABLE_SCHEMA),
    ] {
        let value = serde_json::to_value(&document).unwrap();
        assert_eq!(
            validate_structure(ContractKind::ModelDependencies, &value),
            Ok(())
        );
        assert_eq!(value["contract"], "menura.model_dependencies");
        let decoded: ModelDependenciesDocument = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, document);
    }
}

#[test]
fn dep_2_metrics_carry_their_kind_and_the_document_its_model_hash() {
    let handle = handle_with(CONTEXT_MODEL, CONTEXT_SCHEMA, None, &LoadOptions::default());
    let document = dependencies(&handle).unwrap();
    assert_eq!(&document.model_hash, handle.model().hash());
    assert_eq!(metric(&document, "metric:BILLED").kind, MetricKind::Base);
    assert_eq!(
        metric(&document, "metric:BILLED_CHANGE").kind,
        MetricKind::Derived
    );
    assert_eq!(
        metric(&document, "metric:BILLED_YTD").kind,
        MetricKind::Context
    );
}

#[test]
fn dep_3_1_every_element_is_listed_without_policy_conditions() {
    let handle = handle_with(LINEAGE_MODEL, LINEAGE_SCHEMA, None, &LoadOptions::default());
    let document = dependencies(&handle).unwrap();
    assert_eq!(
        document.datasets.len(),
        handle.model().parts().datasets.len()
    );
    assert_eq!(objects(&document.policies), ["policy:STORES.NORTH_ONLY"]);
    let text = serde_json::to_string(&document).unwrap();
    assert!(!text.contains("NORTH'"), "no condition text");
}

#[test]
fn dep_3_1_the_catalog_state_does_not_change_the_document() {
    let with_state = handle_with(
        SUMMARY_MODEL,
        SUMMARY_SCHEMA,
        Some(SUMMARY_STATE),
        &LoadOptions::default(),
    );
    let without_state = handle_with(SUMMARY_MODEL, SUMMARY_SCHEMA, None, &LoadOptions::default());
    assert_eq!(dependencies(&with_state), dependencies(&without_state));
}

#[test]
fn dep_3_2_lists_are_ordered_and_the_output_is_deterministic() {
    let first = serde_json::to_string(&lineage()).unwrap();
    let second = serde_json::to_string(&lineage()).unwrap();
    assert_eq!(first, second);
    let document = lineage();
    assert_eq!(
        objects(&document.datasets),
        ["dataset:CUSTOMERS", "dataset:ORDERS", "dataset:STORES"]
    );
    let fields = objects(&document.fields);
    let mut sorted = fields.clone();
    sorted.sort_unstable();
    assert_eq!(fields, sorted);
}

#[test]
fn dep_3_2_a_model_metric_comes_before_the_dataset_metrics_of_the_same_first_part() {
    let options = LoadOptions {
        ossie_format: OssieFormat::Development,
        ..LoadOptions::default()
    };
    let handle = handle_with(DEVELOPMENT_MODEL, DEVELOPMENT_SCHEMA, None, &options);
    let document = dependencies(&handle).unwrap();
    let names: Vec<&str> = document
        .metrics
        .iter()
        .map(|entry| entry.object.as_str())
        .collect();
    assert!(names.contains(&"metric:ORDERS.ORDER_COUNT"));
    assert!(names.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn dep_3_3_an_unusable_element_is_listed_with_its_query_error_code() {
    let document = document_of(UNUSABLE_MODEL, UNUSABLE_SCHEMA);
    let field = entry(&document.fields, "field:INVOICES.BILLED_METRIC");
    assert!(!field.usable);
    assert_eq!(field.query_error_code.as_deref(), Some("E0F-02"));
    let unusable = metric(&document, "metric:TWICE_SHARE");
    assert!(!unusable.usable);
    assert_eq!(unusable.query_error_code.as_deref(), Some("E0C-37"));
    assert!(
        unusable
            .model_inputs
            .contains(&"metric:BILLED_TWICE".to_owned())
    );
    let usable = metric(&document, "metric:TOTAL_BILLED");
    assert!(usable.usable);
    assert_eq!(usable.query_error_code, None);
}

#[test]
fn dep_3_4_visibility_inputs_follow_propagation() {
    let document = lineage();
    let orders = entry(&document.datasets, "dataset:ORDERS");
    assert_eq!(
        orders.model_inputs,
        [
            "dataset:STORES",
            "field:ORDERS.STORE_ID",
            "field:STORES.REGION",
            "field:STORES.STORE_ID",
            "relationship:ORDERS_STORES",
        ]
    );
    assert_eq!(
        physical(&orders.physical_inputs),
        [
            "shop.orders.store_id",
            "shop.stores.region",
            "shop.stores.store_id"
        ]
    );
    assert_eq!(orders.security_policies, ["STORES.NORTH_ONLY"]);
    let customers = entry(&document.datasets, "dataset:CUSTOMERS");
    assert!(customers.model_inputs.is_empty());
    assert!(customers.security_policies.is_empty());
}

#[test]
fn dep_3_4_a_relationship_back_to_the_chain_is_not_followed() {
    let document = document_of(PROPAGATION_MODEL, PROPAGATION_SCHEMA);
    let teams = entry(&document.datasets, "dataset:TEAMS");
    assert!(
        teams
            .model_inputs
            .contains(&"relationship:TEAMS_LEADS".to_owned())
    );
    assert!(
        !teams
            .model_inputs
            .contains(&"relationship:PEOPLE_TEAMS".to_owned())
    );
    let events = entry(&document.datasets, "dataset:EVENTS");
    assert_eq!(
        events.security_policies,
        ["CITIES.CITIES_CITY_SCOPE", "VENUES.VENUES_OPENED_SCOPE"]
    );
}

#[test]
fn dep_3_5_a_summary_depends_on_its_base_and_its_policies() {
    let document = document_of(SUMMARY_MODEL, SUMMARY_SCHEMA);
    let summary = entry(&document.datasets, "dataset:ORDERS_BY_MONTH");
    assert!(summary.model_inputs.contains(&"dataset:ORDERS".to_owned()));
    assert!(
        summary
            .model_inputs
            .contains(&"field:ORDERS.ORDER_DATE".to_owned())
    );
    assert!(
        summary
            .model_inputs
            .contains(&"field:ORDERS.AMOUNT".to_owned())
    );
    assert_eq!(
        summary.security_policies,
        entry(&document.datasets, "dataset:ORDERS").security_policies
    );
}

#[test]
fn dep_3_6_a_field_depends_on_its_dataset_and_its_columns() {
    let document = lineage();
    let store_id = entry(&document.fields, "field:STORES.STORE_ID");
    assert_eq!(
        store_id.model_inputs,
        ["dataset:STORES", "field:STORES.REGION"]
    );
    assert_eq!(
        physical(&store_id.physical_inputs),
        ["shop.stores.region", "shop.stores.store_id"]
    );
    assert_eq!(store_id.security_policies, ["STORES.NORTH_ONLY"]);
}

#[test]
fn dep_3_7_a_derived_metric_depends_on_every_metric_it_reads() {
    let document = lineage();
    let derived = metric(&document, "metric:AMOUNT_PER_CUSTOMER");
    for input in [
        "metric:TOTAL_AMOUNT",
        "metric:CUSTOMER_COUNT",
        "field:ORDERS.AMOUNT",
        "field:CUSTOMERS.CUSTOMER_ID",
        "relationship:ORDERS_STORES",
    ] {
        assert!(derived.model_inputs.contains(&input.to_owned()), "{input}");
    }
    assert_eq!(derived.security_policies, ["STORES.NORTH_ONLY"]);
}

#[test]
fn dep_3_7_a_context_metric_depends_on_its_base_and_its_modifier_fields() {
    let document = document_of(CONTEXT_MODEL, CONTEXT_SCHEMA);
    let prior = metric(&document, "metric:BILLED_PRIOR_YEAR");
    assert_eq!(
        prior.model_inputs,
        [
            "dataset:INVOICES",
            "field:INVOICES.BILLED",
            "field:INVOICES.INVOICE_DATE",
            "metric:BILLED"
        ]
    );
    let share = metric(&document, "metric:BILLED_SHARE");
    assert!(
        share
            .model_inputs
            .contains(&"field:CUSTOMERS.REGION".to_owned())
    );
}

#[test]
fn dep_3_7_a_semi_additive_metric_depends_on_its_keys_and_population() {
    let document = document_of(SEMI_ADDITIVE_MODEL, SEMI_ADDITIVE_SCHEMA);
    let carried = metric(&document, "metric:CARRIED_BALANCE");
    for input in [
        "field:BALANCES.PERIOD",
        "field:BALANCES.CUSTOMER_ID",
        "dataset:MEMBERS",
        "field:MEMBERS.VALID_FROM",
        "field:MEMBERS.VALID_TO",
    ] {
        assert!(carried.model_inputs.contains(&input.to_owned()), "{input}");
    }
}

#[test]
fn dep_3_7_a_dataset_metric_depends_on_its_dataset_and_its_source_columns() {
    let options = LoadOptions {
        ossie_format: OssieFormat::Development,
        ..LoadOptions::default()
    };
    let handle = handle_with(DEVELOPMENT_MODEL, DEVELOPMENT_SCHEMA, None, &options);
    let document = dependencies(&handle).unwrap();
    let source = metric(&document, "metric:ORDERS.SOURCE_TOTAL_AMOUNT");
    assert_eq!(source.model_inputs, ["dataset:ORDERS"]);
    assert_eq!(source.physical_inputs.len(), 1);
    let open = metric(&document, "metric:ORDERS.OPEN_ORDER_COUNT");
    assert_eq!(open.model_inputs, ["dataset:ORDERS", "field:ORDERS.STATUS"]);
}

#[test]
fn dep_3_8_relationships_and_policies_have_no_security_policies() {
    let document = lineage();
    let relationship = entry(&document.relationships, "relationship:ORDERS_STORES");
    assert_eq!(
        relationship.model_inputs,
        [
            "dataset:ORDERS",
            "dataset:STORES",
            "field:ORDERS.STORE_ID",
            "field:STORES.REGION",
            "field:STORES.STORE_ID",
        ]
    );
    assert!(relationship.security_policies.is_empty());
    let policy = entry(&document.policies, "policy:STORES.NORTH_ONLY");
    assert_eq!(
        policy.model_inputs,
        ["dataset:STORES", "field:STORES.REGION"]
    );
    assert_eq!(physical(&policy.physical_inputs), ["shop.stores.region"]);
    assert!(policy.security_policies.is_empty());
}

#[test]
fn dep_3_9_summaries_are_not_dependencies_of_metrics() {
    let document = document_of(SUMMARY_MODEL, SUMMARY_SCHEMA);
    let total = metric(&document, "metric:TOTAL_AMOUNT");
    assert!(
        total
            .model_inputs
            .iter()
            .all(|input| !input.contains("ORDERS_BY"))
    );
}

#[cfg(test)]
fn hashes(document: &ModelDependenciesDocument) -> Vec<(String, String)> {
    document
        .datasets
        .iter()
        .chain(&document.relationships)
        .chain(&document.fields)
        .chain(&document.policies)
        .map(|entry| {
            (
                entry.object.clone(),
                entry.definition_hash.as_str().to_owned(),
            )
        })
        .chain(document.metrics.iter().map(|entry| {
            (
                entry.object.clone(),
                entry.definition_hash.as_str().to_owned(),
            )
        }))
        .collect()
}

#[test]
fn dep_3_10_descriptions_do_not_change_definition_hashes() {
    let described = LINEAGE_MODEL.replace(
        "description: 'Orders joined",
        "description: 'Changed: orders joined",
    );
    assert_ne!(described, LINEAGE_MODEL);
    assert_eq!(
        hashes(&lineage()),
        hashes(&document_of(&described, LINEAGE_SCHEMA))
    );
}

#[test]
fn dep_3_10_a_changed_metric_does_not_change_its_dependents() {
    let changed = LINEAGE_MODEL.replace("SUM(orders.amount)", "MAX(orders.amount)");
    assert_ne!(changed, LINEAGE_MODEL);
    let old = lineage();
    let new = document_of(&changed, LINEAGE_SCHEMA);
    assert_ne!(
        metric(&old, "metric:TOTAL_AMOUNT").definition_hash,
        metric(&new, "metric:TOTAL_AMOUNT").definition_hash
    );
    assert_eq!(
        metric(&old, "metric:AMOUNT_PER_CUSTOMER").definition_hash,
        metric(&new, "metric:AMOUNT_PER_CUSTOMER").definition_hash
    );
}

#[cfg(test)]
fn handle_of(model: &str) -> Handle {
    handle_with(model, LINEAGE_SCHEMA, None, &LoadOptions::default())
}

#[cfg(test)]
fn changes(old: &str, new: &str) -> Vec<(String, ChangeKind)> {
    let old = dependencies(&handle_of(old)).unwrap();
    let new = dependencies(&handle_of(new)).unwrap();
    let document = diff(&old, &new).unwrap();
    let value = serde_json::to_value(&document).unwrap();
    assert_eq!(validate_structure(ContractKind::ModelDiff, &value), Ok(()));
    assert_eq!(document.old_model_hash, old.model_hash);
    assert_eq!(document.new_model_hash, new.model_hash);
    document
        .changes
        .into_iter()
        .map(|change| (change.object, change.change))
        .collect()
}

#[test]
fn dep_4_3_identical_models_give_no_changes() {
    assert_eq!(changes(LINEAGE_MODEL, LINEAGE_MODEL), []);
}

#[test]
fn dep_4_2_a_policy_change_is_changed() {
    let changed = LINEAGE_MODEL.replace("region = ''NORTH''", "region = ''SOUTH''");
    assert_ne!(changed, LINEAGE_MODEL);
    assert_eq!(
        changes(LINEAGE_MODEL, &changed),
        [("policy:STORES.NORTH_ONLY".to_owned(), ChangeKind::Changed)]
    );
}

#[test]
fn dep_4_2_a_rename_is_removed_and_added() {
    let renamed = LINEAGE_MODEL.replace("name: orders_customers", "name: orders_clients");
    assert_ne!(renamed, LINEAGE_MODEL);
    assert_eq!(
        changes(LINEAGE_MODEL, &renamed),
        [
            ("relationship:ORDERS_CLIENTS".to_owned(), ChangeKind::Added),
            (
                "relationship:ORDERS_CUSTOMERS".to_owned(),
                ChangeKind::Removed
            ),
        ]
    );
}

#[test]
fn dep_4_1_only_the_changed_definition_is_listed() {
    let changed = LINEAGE_MODEL.replace("SUM(orders.amount)", "MAX(orders.amount)");
    assert_eq!(
        changes(LINEAGE_MODEL, &changed),
        [("metric:TOTAL_AMOUNT".to_owned(), ChangeKind::Changed)]
    );
}

#[test]
fn dep_4_3_changes_are_ordered_by_kind_then_object() {
    let changed = LINEAGE_MODEL
        .replace("name: orders_customers", "name: orders_clients")
        .replace("SUM(orders.amount)", "MAX(orders.amount)")
        .replace("region = ''NORTH''", "region = ''SOUTH''");
    let kinds: Vec<String> = changes(LINEAGE_MODEL, &changed)
        .into_iter()
        .map(|(object, _)| object)
        .collect();
    assert_eq!(
        kinds,
        [
            "relationship:ORDERS_CLIENTS",
            "relationship:ORDERS_CUSTOMERS",
            "metric:TOTAL_AMOUNT",
            "policy:STORES.NORTH_ONLY",
        ]
    );
}

#[test]
fn dep_3_10_the_case_of_a_policy_name_does_not_change_its_hash() {
    let recased = LINEAGE_MODEL.replace(r#""name": "north_only""#, r#""name": "NORTH_ONLY""#);
    assert_ne!(recased, LINEAGE_MODEL);
    assert_eq!(changes(LINEAGE_MODEL, &recased), []);
}

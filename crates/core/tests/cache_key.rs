//! The cache key of the compiled plan: PLN-3, PLN-3.1, PLN-3.2 and PLN-3.3.

mod support;

use menura_contracts::hash_value;
use menura_core::{CompiledPlanDocument, with_state};
use serde_json::json;
use support::{amount_by_region, analyst, compile_default, context, handle, plan, query};

fn compiled_for(context: &str) -> CompiledPlanDocument {
    plan(compile_default(&handle(), &amount_by_region(), context))
}

/// A catalog state of the `shop` schema whose relations are `relations`, as JSON text.
fn state(relations: &str) -> String {
    format!(
        r#"{{"contract": "menura.catalog_state", "contract_version": "1.1.0",
            "document_id": {{"origin": "warehouse:shop", "snapshot": "state-1"}},
            "schema_document_id": {{"origin": "warehouse:shop", "snapshot": "schema-1"}},
            "relations": [{relations}]}}"#
    )
}

/// `orders` partitioned by month, with the periods `2026-09` and `2026-10`.
const PARTITIONED_ORDERS: &str = r#"{"source": "shop.main.orders", "row_count_estimate": 3,
    "partitions": {"column": "order_date", "grain": "month", "versions": [
        {"period": "2026-09", "version": {"origin": "warehouse:shop", "snapshot": "orders-9"}},
        {"period": "2026-10", "version": {"origin": "warehouse:shop", "snapshot": "orders-10"}}]}}"#;

/// `orders` with one version for the whole relation.
const UNPARTITIONED_ORDERS: &str = r#"{"source": "shop.main.orders", "row_count_estimate": 3,
    "version": {"origin": "warehouse:shop", "snapshot": "orders-all"}}"#;

/// `customers` with one version for the whole relation.
const UNPARTITIONED_CUSTOMERS: &str = r#"{"source": "shop.main.customers", "row_count_estimate": 1,
    "version": {"origin": "warehouse:shop", "snapshot": "customers-1"}}"#;

fn compiled_with_state(relations: &str) -> CompiledPlanDocument {
    let handle = with_state(&handle(), state(relations).as_bytes());
    assert!(
        handle.catalog_state().is_some(),
        "the test state is admitted"
    );
    plan(compile_default(&handle, &amount_by_region(), &analyst()))
}

fn data_versions(plan: &CompiledPlanDocument) -> serde_json::Value {
    serde_json::to_value(&plan.cache_key.data_versions).unwrap()
}

#[test]
fn pln_3_the_key_hashes_the_plan_hash_the_fingerprint_and_the_data_versions() {
    let compiled = compiled_for(&analyst());
    let key = &compiled.cache_key;
    let expected = hash_value(&json!({
        "plan_hash": key.plan_hash,
        "security_fingerprint": key.security_fingerprint,
        "data_versions": key.data_versions,
    }));
    assert_eq!(key.key, expected);
}

#[test]
fn pln_3_1_the_plan_hash_covers_dialect_data_source_sql_parameters_and_result_schema() {
    let compiled = compiled_for(&analyst());
    let expected = hash_value(&json!({
        "dialect": compiled.dialect,
        "data_source": compiled.data_source,
        "sql": compiled.sql,
        "parameters": compiled.parameters,
        "result_schema": compiled.result_schema,
    }));
    assert_eq!(compiled.cache_key.plan_hash, expected);
}

#[test]
fn pln_3_2_the_security_fingerprint_is_the_context_fingerprint() {
    let compiled = compiled_for(&analyst());
    let expected = hash_value(&json!({
        "roles": ["analyst"],
        "attributes": {"regions": ["NORTH", "SOUTH"]},
    }));
    assert_eq!(compiled.cache_key.security_fingerprint, expected);
}

#[test]
fn pln_3_2_contexts_that_differ_only_in_subject_share_the_cache_key() {
    let first = compiled_for(&context(
        "user-1",
        &["analyst"],
        r#"{"regions": ["NORTH", "SOUTH"]}"#,
    ));
    let second = compiled_for(&context(
        "user-2",
        &["analyst"],
        r#"{"regions": ["SOUTH", "NORTH"]}"#,
    ));
    assert_eq!(first.cache_key, second.cache_key);
}

#[test]
fn pln_3_2_a_declared_role_changes_the_cache_key() {
    let analyst = compiled_for(&context(
        "user-1",
        &["analyst"],
        r#"{"regions": ["NORTH"]}"#,
    ));
    let both = compiled_for(&context(
        "user-1",
        &["admin", "analyst"],
        r#"{"regions": ["NORTH"]}"#,
    ));
    assert_ne!(
        analyst.cache_key.security_fingerprint,
        both.cache_key.security_fingerprint
    );
    assert_ne!(analyst.cache_key.key, both.cache_key.key);
}

#[test]
fn pln_3_2_an_undeclared_role_keeps_the_cache_key() {
    let analyst = compiled_for(&context(
        "user-1",
        &["analyst"],
        r#"{"regions": ["NORTH"]}"#,
    ));
    let with_other = compiled_for(&context(
        "user-1",
        &["analyst", "auditor"],
        r#"{"regions": ["NORTH"]}"#,
    ));
    assert_eq!(analyst.cache_key, with_other.cache_key);
}

#[test]
fn pln_3_3_data_versions_are_null_without_a_catalog_state() {
    assert_eq!(compiled_for(&analyst()).cache_key.data_versions, None);
}

#[test]
fn pln_3_3_data_versions_list_every_period_of_a_partitioned_relation_read() {
    let compiled = compiled_with_state(&format!("{PARTITIONED_ORDERS}, {UNPARTITIONED_CUSTOMERS}"));
    assert_eq!(
        data_versions(&compiled),
        json!([
            {"source": "shop.main.orders", "period": "2026-09",
             "version": {"origin": "warehouse:shop", "snapshot": "orders-9"}},
            {"source": "shop.main.orders", "period": "2026-10",
             "version": {"origin": "warehouse:shop", "snapshot": "orders-10"}},
        ])
    );
}

#[test]
fn pln_3_3_an_unpartitioned_relation_read_has_the_period_star() {
    let compiled = compiled_with_state(UNPARTITIONED_ORDERS);
    assert_eq!(
        data_versions(&compiled),
        json!([
            {"source": "shop.main.orders", "period": "*",
             "version": {"origin": "warehouse:shop", "snapshot": "orders-all"}},
        ])
    );
}

#[test]
fn pln_3_3_data_versions_are_null_when_the_state_has_no_version_of_a_relation_read() {
    let compiled = compiled_with_state(UNPARTITIONED_CUSTOMERS);
    assert_eq!(compiled.cache_key.data_versions, None);
}

#[test]
fn pln_3_3_data_versions_change_the_key() {
    let without = compiled_for(&analyst());
    let with = compiled_with_state(UNPARTITIONED_ORDERS);
    assert_eq!(without.cache_key.plan_hash, with.cache_key.plan_hash);
    assert_ne!(without.cache_key.key, with.cache_key.key);
}

#[test]
fn pln_3_3_a_dimension_only_query_reads_its_dataset() {
    let handle = with_state(&handle(), state(UNPARTITIONED_ORDERS).as_bytes());
    let dimensions = query(r#"{"dimensions": ["orders.region"]}"#);
    let compiled = plan(compile_default(&handle, &dimensions, &analyst()));
    assert_eq!(
        data_versions(&compiled)[0]["source"],
        json!("shop.main.orders")
    );
}

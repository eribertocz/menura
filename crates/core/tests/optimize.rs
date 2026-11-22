//! E5 between E4 and E6 (E5-2.1, E5-2.3): the catalog state reaches E5, and its key order
//! never changes the compiled plan.

mod support;

use menura_core::LoadOptions;
use support::{
    CATALOG_SCHEMA, MODEL, amount_by_region, analyst, compile_default, handle_with, plan,
};

/// A catalog state of the `shop` model, with a check result.
const STATE: &str = r#"{
  "contract": "menura.catalog_state",
  "contract_version": "1.1.0",
  "document_id": {"origin": "warehouse:shop", "snapshot": "state-1"},
  "schema_document_id": {"origin": "warehouse:shop", "snapshot": "schema-1"},
  "relations": [
    {"source": "shop.main.orders", "row_count_estimate": 3,
     "partitions": {"column": "ORDER_DATE", "grain": "month", "versions": [
       {"period": "2026-09", "version": {"origin": "warehouse:shop", "snapshot": "orders-9"}},
       {"period": "2026-10", "version": {"origin": "warehouse:shop", "snapshot": "orders-10"}}]}},
    {"source": "shop.main.customers", "row_count_estimate": 1,
     "version": {"origin": "warehouse:shop", "snapshot": "customers-1"}}
  ],
  "checks": [
    {"target": "ORDERS_TO_CUSTOMERS", "check": "key_uniqueness",
     "definition_hash": "1111111111111111111111111111111111111111111111111111111111111111",
     "status": "passed", "verified": [
       {"source": "shop.main.customers", "version": {"origin": "warehouse:shop", "snapshot": "customers-1"}},
       {"source": "shop.main.orders", "period": "2026-09", "version": {"origin": "warehouse:shop", "snapshot": "orders-9"}}]}
  ]
}"#;

/// `STATE` with the members of every object, and the elements of every list whose order has no
/// meaning, in another order.
const SHUFFLED_STATE: &str = r#"{
  "checks": [
    {"verified": [
       {"version": {"snapshot": "orders-9", "origin": "warehouse:shop"}, "period": "2026-09", "source": "shop.main.orders"},
       {"version": {"snapshot": "customers-1", "origin": "warehouse:shop"}, "source": "shop.main.customers"}],
     "status": "passed",
     "definition_hash": "1111111111111111111111111111111111111111111111111111111111111111",
     "check": "key_uniqueness", "target": "ORDERS_TO_CUSTOMERS"}
  ],
  "relations": [
    {"version": {"snapshot": "customers-1", "origin": "warehouse:shop"},
     "row_count_estimate": 1, "source": "shop.main.customers"},
    {"partitions": {"versions": [
       {"version": {"snapshot": "orders-10", "origin": "warehouse:shop"}, "period": "2026-10"},
       {"version": {"snapshot": "orders-9", "origin": "warehouse:shop"}, "period": "2026-09"}],
       "grain": "month", "column": "ORDER_DATE"},
     "row_count_estimate": 3, "source": "shop.main.orders"}
  ],
  "schema_document_id": {"snapshot": "schema-1", "origin": "warehouse:shop"},
  "document_id": {"snapshot": "state-1", "origin": "warehouse:shop"},
  "contract_version": "1.1.0",
  "contract": "menura.catalog_state"
}"#;

#[test]
fn e5_2_3_a_shuffled_state_gives_a_byte_identical_plan() {
    let options = LoadOptions::default();
    let first = handle_with(MODEL, CATALOG_SCHEMA, Some(STATE), &options);
    let second = handle_with(MODEL, CATALOG_SCHEMA, Some(SHUFFLED_STATE), &options);
    assert!(first.catalog_state().is_some());
    assert!(second.catalog_state().is_some());

    let from_first = plan(compile_default(&first, &amount_by_region(), &analyst()));
    let from_second = plan(compile_default(&second, &amount_by_region(), &analyst()));

    assert_eq!(from_first.sql.as_bytes(), from_second.sql.as_bytes());
    assert_eq!(from_first, from_second);
}

#[test]
fn e5_2_1_a_rejected_state_compiles_like_no_state() {
    let options = LoadOptions::default();
    let rejected = STATE.replace("\"schema-1\"", "\"schema-2\"");
    let ignored = handle_with(MODEL, CATALOG_SCHEMA, Some(&rejected), &options);
    let without = handle_with(MODEL, CATALOG_SCHEMA, None, &options);
    assert!(ignored.catalog_state().is_none());

    let from_ignored = plan(compile_default(&ignored, &amount_by_region(), &analyst()));
    let from_without = plan(compile_default(&without, &amount_by_region(), &analyst()));

    assert_eq!(from_ignored.sql, from_without.sql);
    assert_eq!(from_ignored.parameters, from_without.parameters);
}

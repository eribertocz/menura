//! Unit tests of the evidence lookup (CAT-7.3, CAT-7.4, CHK-4.1, E5-2.1, E5-5).

use menura_contracts::types::catalog::CatalogStateDocument;
use menura_model::internal_model::{CheckKind, NormalizedName};
use serde_json::{Value, json};

use crate::evidence::Evidence;
use crate::test_support::{CURRENT_HASH, OTHER_HASH, RELATIONSHIP, shop_model};

/// The version token `snapshot` of the test origin.
fn token(snapshot: &str) -> Value {
    json!({"origin": "warehouse:shop", "snapshot": snapshot})
}

/// The `orders` relation, partitioned by month into `2026-09` (`orders-9`) and `2026-10`
/// (`orders-10`).
fn partitioned_orders() -> Value {
    json!({
        "source": "shop.orders",
        "row_count_estimate": 10,
        "partitions": {
            "column": "ORDER_DATE",
            "grain": "month",
            "versions": [
                {"period": "2026-09", "version": token("orders-9")},
                {"period": "2026-10", "version": token("orders-10")}
            ]
        }
    })
}

/// The `customers` relation, not partitioned, at version `customers-1`.
fn customers() -> Value {
    json!({"source": "shop.customers", "row_count_estimate": 2, "version": token("customers-1")})
}

/// The verified versions that match `partitioned_orders` and `customers`.
fn current_verified() -> Value {
    json!([
        {"source": "shop.customers", "version": token("customers-1")},
        {"source": "shop.orders", "version": token("orders-9"), "period": "2026-09"},
        {"source": "shop.orders", "version": token("orders-10"), "period": "2026-10"}
    ])
}

/// A check result of `check` for the test relationship.
fn result(check: &str, hash: &str, status: &str, verified: Value) -> Value {
    let mut result = json!({
        "target": RELATIONSHIP,
        "check": check,
        "definition_hash": hash,
        "status": status
    });
    result["verified"] = verified;
    result
}

/// The state with `relations` and `checks`.
fn state(relations: Value, checks: Value) -> CatalogStateDocument {
    let mut state = json!({
        "contract": "menura.catalog_state",
        "contract_version": "1.1.0",
        "document_id": token("state-1"),
        "schema_document_id": token("schema-1")
    });
    state["relations"] = relations;
    state["checks"] = checks;
    serde_json::from_value(state).expect("a well-formed catalog state")
}

/// The state with both relations and the one check result `check`.
fn state_with(check: Value) -> CatalogStateDocument {
    state(
        json!([partitioned_orders(), customers()]),
        Value::Array(vec![check]),
    )
}

/// Whether `state` holds evidence of `kind` for the test relationship.
fn holds(state: Option<&CatalogStateDocument>, kind: CheckKind) -> bool {
    let model = shop_model();
    Evidence::new(&model, state).holds(kind, &NormalizedName::new(RELATIONSHIP))
}

#[test]
fn cat_7_3_a_passing_check_on_the_current_versions_is_evidence() {
    let state = state_with(result(
        "key_uniqueness",
        CURRENT_HASH,
        "passed",
        current_verified(),
    ));

    assert!(holds(Some(&state), CheckKind::KeyUniqueness));
}

#[test]
fn cat_7_3_evidence_is_for_its_own_kind_only() {
    let state = state_with(result(
        "key_uniqueness",
        CURRENT_HASH,
        "passed",
        current_verified(),
    ));

    assert!(!holds(Some(&state), CheckKind::OrphanKeys));
}

#[test]
fn cat_7_3_an_orphan_keys_check_is_evidence_for_orphan_keys() {
    let state = state_with(result(
        "orphan_keys",
        CURRENT_HASH,
        "passed",
        current_verified(),
    ));

    assert!(holds(Some(&state), CheckKind::OrphanKeys));
}

#[test]
fn cat_7_3_a_failed_check_is_not_evidence() {
    let state = state_with(result(
        "key_uniqueness",
        CURRENT_HASH,
        "failed",
        current_verified(),
    ));

    assert!(!holds(Some(&state), CheckKind::KeyUniqueness));
}

#[test]
fn cat_7_3_a_check_of_another_definition_hash_is_not_evidence() {
    let state = state_with(result(
        "key_uniqueness",
        OTHER_HASH,
        "passed",
        current_verified(),
    ));

    assert!(!holds(Some(&state), CheckKind::KeyUniqueness));
}

#[test]
fn cat_7_3_a_check_of_another_target_is_not_evidence() {
    let mut check = result("key_uniqueness", CURRENT_HASH, "passed", current_verified());
    check["target"] = json!("CUSTOMERS_TO_ORDERS");
    let state = state_with(check);

    assert!(!holds(Some(&state), CheckKind::KeyUniqueness));
}

#[test]
fn cat_7_3_a_missing_verified_period_is_not_evidence() {
    let verified = json!([
        {"source": "shop.customers", "version": token("customers-1")},
        {"source": "shop.orders", "version": token("orders-9"), "period": "2026-09"}
    ]);
    let state = state_with(result("orphan_keys", CURRENT_HASH, "passed", verified));

    assert!(!holds(Some(&state), CheckKind::OrphanKeys));
}

#[test]
fn cat_7_3_a_different_verified_period_version_is_not_evidence() {
    let verified = json!([
        {"source": "shop.customers", "version": token("customers-1")},
        {"source": "shop.orders", "version": token("orders-9"), "period": "2026-09"},
        {"source": "shop.orders", "version": token("orders-10b"), "period": "2026-10"}
    ]);
    let state = state_with(result("orphan_keys", CURRENT_HASH, "passed", verified));

    assert!(!holds(Some(&state), CheckKind::OrphanKeys));
}

#[test]
fn cat_7_3_a_verified_period_the_relation_no_longer_has_is_a_difference() {
    let verified = json!([
        {"source": "shop.customers", "version": token("customers-1")},
        {"source": "shop.orders", "version": token("orders-8"), "period": "2026-08"},
        {"source": "shop.orders", "version": token("orders-9"), "period": "2026-09"},
        {"source": "shop.orders", "version": token("orders-10"), "period": "2026-10"}
    ]);
    let state = state_with(result("orphan_keys", CURRENT_HASH, "passed", verified));

    assert!(!holds(Some(&state), CheckKind::OrphanKeys));
}

#[test]
fn cat_7_3_key_uniqueness_reads_only_the_to_relation() {
    let verified = json!([
        {"source": "shop.customers", "version": token("customers-1")},
        {"source": "shop.orders", "version": token("orders-8"), "period": "2026-08"}
    ]);
    let state = state_with(result("key_uniqueness", CURRENT_HASH, "passed", verified));

    assert!(holds(Some(&state), CheckKind::KeyUniqueness));
}

#[test]
fn cat_7_3_orphan_keys_reads_the_from_relation_too() {
    let verified = json!([
        {"source": "shop.customers", "version": token("customers-1")},
        {"source": "shop.orders", "version": token("orders-8"), "period": "2026-08"}
    ]);
    let state = state_with(result("orphan_keys", CURRENT_HASH, "passed", verified));

    assert!(!holds(Some(&state), CheckKind::OrphanKeys));
}

#[test]
fn cat_7_3_a_missing_non_partitioned_version_is_not_evidence() {
    let verified = json!([
        {"source": "shop.orders", "version": token("orders-9"), "period": "2026-09"},
        {"source": "shop.orders", "version": token("orders-10"), "period": "2026-10"}
    ]);
    let state = state_with(result("key_uniqueness", CURRENT_HASH, "passed", verified));

    assert!(!holds(Some(&state), CheckKind::KeyUniqueness));
}

#[test]
fn cat_7_3_a_different_non_partitioned_version_is_not_evidence() {
    let verified = json!([
        {"source": "shop.customers", "version": token("customers-2")},
        {"source": "shop.orders", "version": token("orders-9"), "period": "2026-09"},
        {"source": "shop.orders", "version": token("orders-10"), "period": "2026-10"}
    ]);
    let state = state_with(result("key_uniqueness", CURRENT_HASH, "passed", verified));

    assert!(!holds(Some(&state), CheckKind::KeyUniqueness));
}

#[test]
fn cat_7_3_a_period_on_a_non_partitioned_relation_is_a_difference() {
    let verified = json!([
        {"source": "shop.customers", "version": token("customers-1"), "period": "2026-10"},
        {"source": "shop.orders", "version": token("orders-9"), "period": "2026-09"},
        {"source": "shop.orders", "version": token("orders-10"), "period": "2026-10"}
    ]);
    let state = state_with(result("key_uniqueness", CURRENT_HASH, "passed", verified));

    assert!(!holds(Some(&state), CheckKind::KeyUniqueness));
}

#[test]
fn cat_7_3_two_non_partitioned_relations_need_both_versions() {
    let orders = json!({"source": "shop.orders", "row_count_estimate": 10, "version": token("orders-1")});
    let verified = json!([
        {"source": "shop.customers", "version": token("customers-1")},
        {"source": "shop.orders", "version": token("orders-1")}
    ]);
    let check = result("key_uniqueness", CURRENT_HASH, "passed", verified);
    let state = state(json!([orders, customers()]), json!([check]));

    assert!(holds(Some(&state), CheckKind::KeyUniqueness));
}

#[test]
fn cat_7_3_a_relation_missing_from_the_state_has_no_current_version() {
    let check = result("key_uniqueness", CURRENT_HASH, "passed", current_verified());
    let state = state(json!([partitioned_orders()]), json!([check]));

    assert!(!holds(Some(&state), CheckKind::KeyUniqueness));
}

#[test]
fn cat_7_3_one_passing_result_among_several_is_evidence() {
    let failed = result("key_uniqueness", CURRENT_HASH, "failed", current_verified());
    let passing = result("key_uniqueness", CURRENT_HASH, "passed", current_verified());
    let state = state(
        json!([partitioned_orders(), customers()]),
        json!([failed, passing]),
    );

    assert!(holds(Some(&state), CheckKind::KeyUniqueness));
}

#[test]
fn cat_7_4_tokens_are_compared_by_both_fields_never_interpreted() {
    let verified = json!([
        {"source": "shop.customers", "version": {"origin": "warehouse:other", "snapshot": "customers-1"}},
        {"source": "shop.orders", "version": token("orders-9"), "period": "2026-09"},
        {"source": "shop.orders", "version": token("orders-10"), "period": "2026-10"}
    ]);
    let state = state_with(result("key_uniqueness", CURRENT_HASH, "passed", verified));

    assert!(!holds(Some(&state), CheckKind::KeyUniqueness));
}

#[test]
fn cat_7_4_a_numeric_looking_token_is_not_parsed() {
    let customers =
        json!({"source": "shop.customers", "row_count_estimate": 2, "version": token("10")});
    let verified = json!([
        {"source": "shop.customers", "version": token("010")},
        {"source": "shop.orders", "version": token("orders-9"), "period": "2026-09"},
        {"source": "shop.orders", "version": token("orders-10"), "period": "2026-10"}
    ]);
    let check = result("key_uniqueness", CURRENT_HASH, "passed", verified);
    let state = state(json!([partitioned_orders(), customers]), json!([check]));

    assert!(!holds(Some(&state), CheckKind::KeyUniqueness));
}

#[test]
fn chk_4_1_e5_2_1_without_a_state_there_is_no_evidence() {
    assert!(!holds(None, CheckKind::KeyUniqueness));
    assert!(!holds(None, CheckKind::OrphanKeys));
}

#[test]
fn e5_5_a_state_without_checks_is_no_evidence() {
    let state = state(json!([partitioned_orders(), customers()]), json!([]));

    assert!(!holds(Some(&state), CheckKind::KeyUniqueness));
}

#[test]
fn chk_4_1_a_relationship_without_a_recorded_check_has_no_evidence() {
    let state = state_with(result(
        "key_uniqueness",
        CURRENT_HASH,
        "passed",
        current_verified(),
    ));
    let model = shop_model();

    let unknown = NormalizedName::new("CUSTOMERS_TO_ORDERS");
    assert!(!Evidence::new(&model, Some(&state)).holds(CheckKind::KeyUniqueness, &unknown));
}

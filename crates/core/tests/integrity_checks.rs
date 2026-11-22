//! `integrity_checks` over the `chk-relationships` conformance model: the `key_uniqueness`
//! and `orphan_keys` checks of each relationship (E0H-4.3, CHK-2, CHK-6.1, CHK-6.2).

#![cfg(test)]

use menura_contracts::types::descriptions::{CheckKind, IntegrityCheck, IntegrityChecksContract};
use menura_core::{Dialect, Handle, IntegrityChecksDocument, LoadOptions, integrity_checks, load};

const MODEL: &str = include_str!("../../../conformance/models/chk-relationships/model.yaml");
const CATALOG_SCHEMA: &str =
    include_str!("../../../conformance/models/chk-relationships/catalog-schema.json");

fn handle() -> Handle {
    load(
        MODEL.as_bytes(),
        CATALOG_SCHEMA.as_bytes(),
        None,
        &LoadOptions::default(),
    )
    .unwrap()
}

fn document() -> IntegrityChecksDocument {
    integrity_checks(&handle(), Dialect::Duckdb).unwrap()
}

fn check(id: &str) -> IntegrityCheck {
    document()
        .checks
        .into_iter()
        .find(|check| check.check_id == id)
        .unwrap()
}

#[test]
fn chk_2_document_names_the_contract_model_and_dialect() {
    let handle = handle();
    let document = integrity_checks(&handle, Dialect::Duckdb).unwrap();
    assert_eq!(document.contract, IntegrityChecksContract::IntegrityChecks);
    assert_eq!(document.contract_version.to_string(), "1.2.0");
    assert_eq!(document.model_hash, *handle.model().hash());
    assert_eq!(document.dialect, Dialect::Duckdb);
}

#[test]
fn e0h_4_3_one_check_of_each_relationship_kind_sorted_by_check_id() {
    let ids: Vec<String> = document()
        .checks
        .into_iter()
        .map(|check| check.check_id)
        .collect();
    assert_eq!(
        ids,
        [
            "key_uniqueness:INVOICES_CUSTOMER",
            "key_uniqueness:INVOICES_STORE",
            "orphan_keys:INVOICES_CUSTOMER",
            "orphan_keys:INVOICES_STORE",
        ]
    );
}

#[test]
fn chk_2_each_check_has_its_kind_and_target() {
    let check = check("orphan_keys:INVOICES_CUSTOMER");
    assert_eq!(check.kind, CheckKind::OrphanKeys);
    assert_eq!(check.target, "INVOICES_CUSTOMER");
    assert!(check.build.is_none());
}

#[test]
fn chk_3_definition_hash_is_the_recorded_hash() {
    let handle = handle();
    let document = integrity_checks(&handle, Dialect::Duckdb).unwrap();
    for generated in &document.checks {
        let recorded = handle
            .model()
            .parts()
            .checks
            .iter()
            .find(|recorded| recorded.check_id == generated.check_id)
            .unwrap();
        assert_eq!(generated.definition_hash, recorded.definition_hash);
    }
}

#[test]
fn chk_2_4_sources_are_the_relations_read_sorted() {
    assert_eq!(
        check("key_uniqueness:INVOICES_CUSTOMER").sources,
        ["shop.main.customers"]
    );
    assert_eq!(
        check("orphan_keys:INVOICES_CUSTOMER").sources,
        ["shop.main.customers", "shop.main.invoices"]
    );
}

#[test]
fn chk_4_2_check_queries_come_from_the_generator_of_e6() {
    for check in document().checks {
        assert!(
            check.sql.starts_with("WITH\n  \"menura_1_scan\" AS ("),
            "{}",
            check.sql
        );
    }
}

#[test]
fn chk_2_2_no_row_policy_applies_to_a_check() {
    for check in document().checks {
        assert!(!check.sql.contains("NORTH"), "{}", check.sql);
    }
}

#[test]
fn chk_2_5_a_check_query_has_no_limit() {
    for check in document().checks {
        assert!(!check.sql.contains("LIMIT"), "{}", check.sql);
    }
}

#[test]
fn chk_7_1_relationship_checks_bind_no_slot() {
    for check in document().checks {
        assert!(check.parameters.is_empty(), "{:?}", check.parameters);
    }
}

#[test]
fn chk_7_2_no_binding_appears_in_a_relationship_check() {
    let text = serde_json::to_string(&document()).unwrap();
    assert!(!text.contains("binding"));
}

#[test]
fn chk_7_3_a_relationship_check_has_no_time_zone_slot() {
    for check in document().checks {
        assert!(!check.sql.contains("AT TIME ZONE"), "{}", check.sql);
    }
}

#[test]
fn chk_7_4_only_summary_checks_have_period_slots() {
    for check in document().checks {
        assert!(!check.sql.contains("from_day"), "{}", check.sql);
        assert!(check.build.is_none());
    }
}

#[test]
fn chk_6_1_key_uniqueness_counts_complete_groups_with_more_than_one_row() {
    let sql = check("key_uniqueness:INVOICES_CUSTOMER").sql;
    assert!(sql.contains("\"CUSTOMER_ID\" IS NOT NULL"), "{sql}");
    assert!(
        sql.contains("CAST(COUNT(*) AS BIGINT) AS \"menura.row_count\""),
        "{sql}"
    );
    assert!(sql.contains("\"menura.row_count\" > 1"), "{sql}");
    assert!(
        sql.ends_with("SELECT\n  \"CUSTOMER_ID\",\n  \"row_count\"\nFROM \"menura_5_project\""),
        "{sql}"
    );
}

#[test]
fn chk_6_2_orphan_keys_is_an_anti_join_of_complete_keys() {
    let sql = check("orphan_keys:INVOICES_CUSTOMER").sql;
    assert!(sql.contains("\"CUSTOMER_ID\" IS NOT NULL"), "{sql}");
    assert!(sql.contains("LEFT JOIN"), "{sql}");
    assert!(
        sql.contains("\"INVOICES_CUSTOMER.CUSTOMER_ID\" IS NULL"),
        "{sql}"
    );
    assert!(
        sql.ends_with("SELECT\n  \"CUSTOMER_ID\"\nFROM \"menura_6_project\""),
        "{sql}"
    );
}

#[test]
fn chk_6_2_a_source_column_key_is_named_as_the_catalog_spells_it() {
    let sql = check("orphan_keys:INVOICES_STORE").sql;
    assert!(
        sql.contains("\"store_code\" AS \"menura.source.STORE_CODE\""),
        "{sql}"
    );
    assert!(
        sql.ends_with("SELECT\n  \"store_code\"\nFROM \"menura_6_project\""),
        "{sql}"
    );
}

#[test]
fn chk_2_the_same_model_gives_the_same_document() {
    assert_eq!(document(), document());
}

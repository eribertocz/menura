//! `load` and `integrity_checks` over the `val-temporal` conformance model: a temporal
//! relationship is an edge without `W0E-02` (VAL-2.12), and gets one `temporal_orphans` check
//! and no `key_uniqueness` or `orphan_keys` check (VAL-2.16), whose query binds no
//! `time_zone` in either dialect (VAL-2.16a, CHK-7.3).

#![cfg(test)]

use menura_contracts::types::descriptions::CheckKind;
use menura_core::{Dialect, Handle, IntegrityChecksDocument, LoadOptions, integrity_checks, load};

const MODEL: &str = include_str!("../../../conformance/models/val-temporal/model.yaml");
const CATALOG_SCHEMA: &str =
    include_str!("../../../conformance/models/val-temporal/catalog-schema.json");

fn handle() -> Handle {
    load(
        MODEL.as_bytes(),
        CATALOG_SCHEMA.as_bytes(),
        None,
        &LoadOptions::default(),
    )
    .unwrap()
}

fn document(dialect: Dialect) -> IntegrityChecksDocument {
    integrity_checks(&handle(), dialect).unwrap()
}

#[test]
fn val_2_12_a_temporal_relationship_gets_no_w0e_02() {
    let handle = handle();
    let undeclared: Vec<String> = handle
        .warnings()
        .filter(|warning| warning.code == "W0E-02")
        .map(|warning| warning.location.clone().unwrap().pointer)
        .collect();
    assert_eq!(undeclared, ["/semantic_model/0/relationships/1/to_columns"]);
    assert!(handle.warnings().any(|warning| warning.code == "W0E-01"));
}

#[test]
fn val_2_16_a_temporal_relationship_has_one_temporal_orphans_check() {
    let checks: Vec<(CheckKind, String)> = document(Dialect::Duckdb)
        .checks
        .into_iter()
        .map(|check| (check.kind, check.target))
        .collect();
    let of = |kind: CheckKind, target: &str| (kind, target.to_owned());
    assert_eq!(
        checks,
        [
            of(CheckKind::KeyUniqueness, "SALE_CUSTOMER_ANY"),
            of(CheckKind::OrphanKeys, "SALE_CUSTOMER_ANY"),
            of(CheckKind::Population, "CUSTOMER_HISTORY"),
            of(CheckKind::Population, "TIER_HISTORY"),
            of(CheckKind::TemporalOrphans, "SALE_CUSTOMER"),
            of(CheckKind::TemporalOrphans, "VISIT_TIER"),
        ]
    );
}

#[test]
fn val_2_16a_chk_7_3_the_temporal_orphans_query_binds_no_time_zone_in_either_dialect() {
    for dialect in [Dialect::Duckdb, Dialect::Clickhouse] {
        for check in document(dialect)
            .checks
            .into_iter()
            .filter(|check| check.kind == CheckKind::TemporalOrphans)
        {
            assert!(
                check.parameters.is_empty(),
                "{dialect:?} {}",
                check.check_id
            );
            assert!(!check.sql.contains("ASOF"), "{}", check.sql);
        }
    }
}

#[test]
fn val_2_16a_the_query_returns_the_from_columns_then_from_time() {
    let check = document(Dialect::Duckdb)
        .checks
        .into_iter()
        .find(|check| check.check_id == "temporal_orphans:SALE_CUSTOMER")
        .unwrap();
    let last = check.sql.rsplit("\nSELECT\n").next().unwrap();
    assert!(
        last.starts_with(
            "  \"CUSTOMER_ID\" AS \"CUSTOMER_ID\",\n  \"SOLD_ON\" AS \"SOLD_ON\"\nFROM"
        ),
        "{last}"
    );
    assert_eq!(check.sources, ["shop.customer_versions", "shop.sales"]);
}

#[test]
fn val_2_20_an_unusable_from_time_valid_from_or_valid_to_makes_the_relationship_unusable() {
    let model = include_str!("../../../conformance/models/val-temporal-unusable/model.yaml");
    let catalog_schema =
        include_str!("../../../conformance/models/val-temporal-unusable/catalog-schema.json");
    let handle = load(
        model.as_bytes(),
        catalog_schema.as_bytes(),
        None,
        &LoadOptions::default(),
    )
    .unwrap();
    let parts = handle.model().parts();
    let unusable: Vec<(&str, Vec<&str>)> = parts
        .relationships
        .iter()
        .map(|relationship| {
            (
                relationship.name.as_str(),
                relationship
                    .query_errors
                    .iter()
                    .map(menura_model::internal_model::DiagnosticCode::as_str)
                    .collect(),
            )
        })
        .collect();
    assert_eq!(
        unusable,
        [
            ("SALE_CUSTOMER_BY_DAY", vec!["E0D-03"]),
            ("SALE_CUSTOMER_FROM", vec!["E0D-03"]),
            ("SALE_CUSTOMER_TO", vec!["E0D-03"]),
        ]
    );
    assert!(parts.graph.edges.is_empty());
    assert!(
        parts
            .checks
            .iter()
            .all(|check| check.kind.as_str() == "population"),
        "{:?}",
        parts.checks
    );
}

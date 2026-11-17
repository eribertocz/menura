//! Admission of the catalog schema and catalog state documents: CAT-2, CAT-3, CAT-4.2, CAT-6.1,
//! CAT-8 and CAT-9.

use semantic_compiler_contracts::types::catalog::{CatalogSchemaDocument, RelationData};
use semantic_compiler_contracts::types::common::{LogicalType, Token};
use semantic_compiler_contracts::{
    AdmissionCode, AdmissionError, admit_catalog_schema, admit_catalog_state,
};
use serde_json::{Value, json};

// `cfg(test)` marks the helpers as test code, where `expect` is allowed (clippy.toml).

#[cfg(test)]
fn schema_example() -> Value {
    parse(include_str!(
        "../../../spec/contracts/examples/catalog-schema.example.json"
    ))
}

#[cfg(test)]
fn state_example() -> Value {
    parse(include_str!(
        "../../../spec/contracts/examples/catalog-state.example.json"
    ))
}

#[cfg(test)]
fn parse(text: &str) -> Value {
    serde_json::from_str(text).expect("examples are JSON")
}

/// Replaces the value at `pointer` (which must exist) with `value`.
#[cfg(test)]
fn with(mut document: Value, pointer: &str, value: Value) -> Value {
    *document.pointer_mut(pointer).expect("pointer exists") = value;
    document
}

/// Adds the member `name` to the object at `pointer`.
#[cfg(test)]
fn adding(mut document: Value, pointer: &str, name: &str, value: Value) -> Value {
    document
        .pointer_mut(pointer)
        .and_then(Value::as_object_mut)
        .expect("target is an object")
        .insert(name.to_owned(), value);
    document
}

#[cfg(test)]
fn admit_schema(document: &Value) -> Result<CatalogSchemaDocument, AdmissionError> {
    admit_catalog_schema(&document.to_string())
}

#[cfg(test)]
fn admitted_schema() -> CatalogSchemaDocument {
    admit_schema(&schema_example()).expect("the schema example is admitted")
}

/// The code and pointer of the first failure of a schema document.
#[cfg(test)]
fn schema_failure(document: &Value) -> (AdmissionCode, String) {
    let error = admit_schema(document).expect_err("the schema document is rejected");
    (error.code, error.pointer)
}

/// The code and pointer of the first failure of a state document, against the schema example.
#[cfg(test)]
fn state_failure(document: &Value) -> (AdmissionCode, String) {
    let error = admit_catalog_state(&document.to_string(), &admitted_schema())
        .expect_err("the state document is ignored");
    (error.code, error.pointer)
}

#[cfg(test)]
fn assert_state_admitted(document: &Value) {
    let admitted = admit_catalog_state(&document.to_string(), &admitted_schema());
    assert!(admitted.is_ok(), "{admitted:?}");
}

#[cfg(test)]
fn pointer(text: &str) -> String {
    text.to_owned()
}

// CAT-2: two documents.

#[test]
fn cat_2_1_each_document_is_admitted_with_its_own_document_id() {
    let schema = admitted_schema();
    let state = admit_catalog_state(&state_example().to_string(), &schema)
        .expect("the state example is admitted");
    assert_eq!(schema.document_id.snapshot, "schema-0042");
    assert_eq!(state.document_id.snapshot, "state-9120");
    assert_ne!(schema.document_id, state.document_id);
}

#[test]
fn cat_2_2_a_state_for_another_schema_is_ignored_with_wcat_04() {
    let state = with(
        state_example(),
        "/schema_document_id/snapshot",
        json!("schema-0043"),
    );
    assert_eq!(
        state_failure(&state),
        (AdmissionCode::Wcat04, pointer("/schema_document_id"))
    );
}

#[test]
fn cat_2_3_the_schema_is_admitted_without_any_state() {
    // Schema admission takes no state document, so a state can never change its outcome.
    let schema = admitted_schema();
    let broken_state = with(state_example(), "/relations", json!("not a list"));
    let error =
        admit_catalog_state(&broken_state.to_string(), &schema).expect_err("the state is ignored");
    assert!(error.code.is_warning());
    assert_eq!(admitted_schema(), schema);
}

// CAT-3: format and validation.

#[test]
fn cat_3_1_structure_is_checked_before_the_semantic_rules() {
    // A repeated source (CAT-8.1) and a schema violation: the schema violation is reported.
    let relations = schema_example()["relations"].clone();
    let mut repeated = relations.as_array().expect("list").clone();
    repeated.push(relations[0].clone());
    let document = with(schema_example(), "/relations", Value::Array(repeated));
    let document = with(document, "/engine/name", json!(42));
    assert_eq!(
        schema_failure(&document),
        (AdmissionCode::Ecat01, pointer("/engine/name"))
    );
}

#[test]
fn cat_3_2_a_newer_minor_version_is_rejected_with_ecat_02_and_wcat_02() {
    let schema = with(schema_example(), "/contract_version", json!("1.1.0"));
    assert_eq!(
        schema_failure(&schema),
        (AdmissionCode::Ecat02, pointer("/contract_version"))
    );
    let state = with(state_example(), "/contract_version", json!("2.0.0"));
    assert_eq!(
        state_failure(&state),
        (AdmissionCode::Wcat02, pointer("/contract_version"))
    );
}

#[test]
fn cat_3_2_a_patch_version_is_accepted() {
    let schema = with(schema_example(), "/contract_version", json!("1.0.7"));
    assert!(admit_schema(&schema).is_ok());
}

#[test]
fn cat_3_3_an_unknown_member_is_rejected_with_ecat_01_and_wcat_01() {
    let schema = adding(schema_example(), "/relations/0", "comment", json!("x"));
    assert_eq!(
        schema_failure(&schema),
        (AdmissionCode::Ecat01, pointer("/relations/0/comment"))
    );
    let state = adding(state_example(), "/relations/1", "comment", json!("x"));
    assert_eq!(
        state_failure(&state),
        (AdmissionCode::Wcat01, pointer("/relations/1/comment"))
    );
}

// CAT-4.2 and CAT-6.1.

#[test]
fn cat_4_2_logical_types_are_kept_exactly_as_declared() {
    let document = with(
        schema_example(),
        "/relations/0/columns/5/logical_type",
        json!({"kind": "decimal", "precision": 38, "scale": 38}),
    );
    let schema = admit_schema(&document).expect("scale equal to precision is admitted");
    assert_eq!(
        schema.relations[0].columns[5].logical_type,
        LogicalType::Decimal {
            precision: 38,
            scale: 38
        }
    );
}

#[test]
fn cat_6_1_tokens_that_look_numeric_are_compared_as_text() {
    let schema = with(schema_example(), "/document_id/snapshot", json!("0042"));
    let schema = admit_schema(&schema).expect("schema is admitted");
    let state = with(state_example(), "/schema_document_id/snapshot", json!("42"));
    let error =
        admit_catalog_state(&state.to_string(), &schema).expect_err("the tokens are different");
    assert_eq!(error.code, AdmissionCode::Wcat04);
    let eight = Token {
        origin: "warehouse".to_owned(),
        snapshot: "8".to_owned(),
    };
    let zero_eight = Token {
        snapshot: "08".to_owned(),
        ..eight.clone()
    };
    assert_ne!(eight, zero_eight);
}

// CAT-8: semantic rules of the schema document.

#[test]
fn cat_8_1_a_repeated_source_in_the_schema_is_ecat_03() {
    let document = with(
        schema_example(),
        "/relations/2/source",
        json!("sales.public.fact_invoice"),
    );
    assert_eq!(
        schema_failure(&document),
        (AdmissionCode::Ecat03, pointer("/relations/2/source"))
    );
}

#[test]
fn cat_8_2_a_repeated_column_name_is_ecat_03() {
    let document = with(
        schema_example(),
        "/relations/1/columns/2/name",
        json!("customer_id"),
    );
    assert_eq!(
        schema_failure(&document),
        (
            AdmissionCode::Ecat03,
            pointer("/relations/1/columns/2/name")
        )
    );
}

#[test]
fn cat_8_2_column_names_that_differ_only_in_case_are_ecat_03() {
    let document = with(
        schema_example(),
        "/relations/1/columns/2/name",
        json!("CUSTOMER_ID"),
    );
    assert_eq!(
        schema_failure(&document),
        (
            AdmissionCode::Ecat03,
            pointer("/relations/1/columns/2/name")
        )
    );
}

#[test]
fn cat_8_2_amount_and_upper_case_amount_in_one_relation_are_ecat_03() {
    let document = with(
        schema_example(),
        "/relations/0/columns/6/name",
        json!("AMOUNT"),
    );
    assert_eq!(
        schema_failure(&document),
        (
            AdmissionCode::Ecat03,
            pointer("/relations/0/columns/6/name")
        )
    );
}

#[test]
fn cat_8_2_the_case_column_conformance_schema_is_ecat_03() {
    let document = parse(include_str!(
        "../../../conformance/models/cat-schema-case-column-001/catalog-schema.json"
    ));
    assert_eq!(
        schema_failure(&document),
        (
            AdmissionCode::Ecat03,
            pointer("/relations/0/columns/4/name")
        )
    );
}

#[test]
fn cat_8_2_non_ascii_letters_are_compared_after_upper_case_mapping() {
    let region = with(
        schema_example(),
        "/relations/1/columns/1/name",
        json!("r\u{e9}gion"),
    );
    let document = with(region, "/relations/1/columns/2/name", json!("R\u{c9}GION"));
    assert_eq!(
        schema_failure(&document),
        (
            AdmissionCode::Ecat03,
            pointer("/relations/1/columns/2/name")
        )
    );
}

#[test]
fn cat_8_2_the_simple_mapping_keeps_the_length_of_a_name() {
    // The full mapping of `ß` is `SS`; the simple mapping keeps `ß` (CON-5).
    let street = with(
        schema_example(),
        "/relations/1/columns/1/name",
        json!("stra\u{df}e"),
    );
    let document = with(street, "/relations/1/columns/2/name", json!("STRASSE"));
    assert!(admit_schema(&document).is_ok());
}

#[test]
fn cat_8_2_iota_subscript_letters_use_their_simple_upper_case_mapping() {
    // U+1FB3 has the full mapping `ΑΙ` and the simple mapping U+1FBC.
    let small = with(
        schema_example(),
        "/relations/1/columns/1/name",
        json!("\u{1fb3}"),
    );
    let document = with(small, "/relations/1/columns/2/name", json!("\u{1fbc}"));
    assert_eq!(
        schema_failure(&document),
        (
            AdmissionCode::Ecat03,
            pointer("/relations/1/columns/2/name")
        )
    );
}

#[test]
fn cat_8_2_the_same_column_name_in_two_relations_is_admitted() {
    // `customer_id` is a column of three relations of the example.
    assert!(admit_schema(&schema_example()).is_ok());
}

#[test]
fn cat_8_3_a_scale_above_the_precision_is_ecat_03() {
    let document = with(
        schema_example(),
        "/relations/0/columns/5/logical_type/scale",
        json!(19),
    );
    assert_eq!(
        schema_failure(&document),
        (
            AdmissionCode::Ecat03,
            pointer("/relations/0/columns/5/logical_type/scale")
        )
    );
}

#[test]
fn cat_8_3_a_list_element_decimal_is_checked() {
    let document = with(
        schema_example(),
        "/relations/1/columns/1/logical_type",
        json!({"kind": "list", "element": {"kind": "decimal", "precision": 4, "scale": 5}}),
    );
    assert_eq!(
        schema_failure(&document),
        (
            AdmissionCode::Ecat03,
            pointer("/relations/1/columns/1/logical_type/element/scale")
        )
    );
}

// CAT-8: semantic rules of the state document.

#[test]
fn cat_8_1_a_repeated_source_in_the_state_is_wcat_03() {
    let document = with(
        state_example(),
        "/relations/1/source",
        json!("sales.public.fact_invoice"),
    );
    assert_eq!(
        state_failure(&document),
        (AdmissionCode::Wcat03, pointer("/relations/1/source"))
    );
}

#[test]
fn cat_8_4_a_relation_source_missing_from_the_schema_is_wcat_03() {
    let document = with(
        state_example(),
        "/relations/1/source",
        json!("sales.public.dim_store"),
    );
    assert_eq!(
        state_failure(&document),
        (AdmissionCode::Wcat03, pointer("/relations/1/source"))
    );
}

#[test]
fn cat_8_4_a_check_source_missing_from_the_schema_is_wcat_03() {
    let document = with(
        state_example(),
        "/checks/0/verified/2/source",
        json!("sales.public.dim_store"),
    );
    assert_eq!(
        state_failure(&document),
        (
            AdmissionCode::Wcat03,
            pointer("/checks/0/verified/2/source")
        )
    );
}

#[test]
fn cat_8_5_a_partition_column_missing_from_the_relation_is_wcat_03() {
    let document = with(
        state_example(),
        "/relations/0/partitions/column",
        json!("created_at"),
    );
    assert_eq!(
        state_failure(&document),
        (
            AdmissionCode::Wcat03,
            pointer("/relations/0/partitions/column")
        )
    );
}

#[test]
fn cat_8_5_a_partition_column_that_is_not_a_date_is_wcat_03() {
    let document = with(
        state_example(),
        "/relations/0/partitions/column",
        json!("status"),
    );
    assert_eq!(
        state_failure(&document),
        (
            AdmissionCode::Wcat03,
            pointer("/relations/0/partitions/column")
        )
    );
}

#[test]
fn cat_8_5_a_timestamp_partition_column_is_admitted() {
    let schema = with(
        schema_example(),
        "/relations/0/columns/1/logical_type",
        json!({"kind": "timestamp", "precision": "us", "with_time_zone": true}),
    );
    let schema = admit_schema(&schema).expect("schema is admitted");
    assert!(admit_catalog_state(&state_example().to_string(), &schema).is_ok());
}

#[test]
fn cat_8_6_a_period_of_another_grain_is_wcat_03() {
    let document = with(
        state_example(),
        "/relations/0/partitions/versions/1/period",
        json!("2026-10-01"),
    );
    assert_eq!(
        state_failure(&document),
        (
            AdmissionCode::Wcat03,
            pointer("/relations/0/partitions/versions/1/period")
        )
    );
}

#[test]
fn cat_8_6_a_date_that_does_not_exist_is_wcat_03() {
    let document = with(
        state_example(),
        "/relations/0/partitions/grain",
        json!("day"),
    );
    let document = with(
        document,
        "/relations/0/partitions/versions",
        json!([{"period": "2026-02-29", "version": {"origin": "o", "snapshot": "1"}}]),
    );
    assert_eq!(
        state_failure(&document),
        (
            AdmissionCode::Wcat03,
            pointer("/relations/0/partitions/versions/0/period")
        )
    );
}

#[test]
fn cat_8_6_week_53_exists_only_in_long_iso_years() {
    let weeks = |period: &str| {
        let document = with(
            state_example(),
            "/relations/0/partitions/grain",
            json!("week"),
        );
        with(
            document,
            "/relations/0/partitions/versions",
            json!([{"period": period, "version": {"origin": "o", "snapshot": "1"}}]),
        )
    };
    // 2021-01-03 is in week 2020-W53 (CAT-5.1); 2026 has 53 weeks too, 2025 does not.
    assert_state_admitted(&weeks("2020-W53"));
    assert_state_admitted(&weeks("2026-W53"));
    assert_eq!(
        state_failure(&weeks("2025-W53")),
        (
            AdmissionCode::Wcat03,
            pointer("/relations/0/partitions/versions/0/period")
        )
    );
}

#[test]
fn cat_8_6_a_repeated_partition_period_is_wcat_03() {
    let document = with(
        state_example(),
        "/relations/0/partitions/versions/1/period",
        json!("2026-09"),
    );
    assert_eq!(
        state_failure(&document),
        (
            AdmissionCode::Wcat03,
            pointer("/relations/0/partitions/versions/1/period")
        )
    );
}

#[test]
fn cat_8_6_a_repeated_built_from_period_is_wcat_03() {
    let document = with(
        state_example(),
        "/relations/2/aggregate_build/built_from/1/period",
        json!("2026-09"),
    );
    assert_eq!(
        state_failure(&document),
        (
            AdmissionCode::Wcat03,
            pointer("/relations/2/aggregate_build/built_from/1/period")
        )
    );
}

#[test]
fn cat_8_6_summary_periods_of_mixed_grains_are_wcat_03() {
    let document = with(
        state_example(),
        "/relations/2/aggregate_build/integrity_check/verified/0/period",
        json!("2026-Q3"),
    );
    assert_eq!(
        state_failure(&document),
        (
            AdmissionCode::Wcat03,
            pointer("/relations/2/aggregate_build/integrity_check/verified/0/period")
        )
    );
}

#[test]
fn cat_8_6_a_repeated_check_version_is_wcat_03() {
    let document = with(
        state_example(),
        "/checks/0/verified/1/period",
        json!("2026-09"),
    );
    assert_eq!(
        state_failure(&document),
        (
            AdmissionCode::Wcat03,
            pointer("/checks/0/verified/1/period")
        )
    );
}

#[test]
fn cat_8_7_a_coverage_that_ends_before_it_starts_is_wcat_03() {
    let document = with(
        state_example(),
        "/relations/2/aggregate_build/coverage/from",
        json!("2026-11"),
    );
    assert_eq!(
        state_failure(&document),
        (
            AdmissionCode::Wcat03,
            pointer("/relations/2/aggregate_build/coverage/to")
        )
    );
}

#[test]
fn cat_8_7_a_coverage_of_one_period_is_admitted() {
    let document = with(
        state_example(),
        "/relations/2/aggregate_build/coverage/to",
        json!("2026-09"),
    );
    assert_state_admitted(&document);
}

#[test]
fn cat_8_9_periods_without_rows_are_not_listed() {
    // A gap between listed periods, and a partitioned relation without any row, are admitted.
    let gap = with(
        state_example(),
        "/relations/0/partitions/versions/1/period",
        json!("2026-12"),
    );
    assert_state_admitted(&gap);
    let empty = with(
        state_example(),
        "/relations/0/partitions/versions",
        json!([]),
    );
    let state = admit_catalog_state(&empty.to_string(), &admitted_schema()).expect("admitted");
    let RelationData::Partitions(partitions) = &state.relations[0].data else {
        panic!("the relation is partitioned");
    };
    assert!(partitions.versions.is_empty());
}

// CAT-9: error codes and their order.

#[test]
fn cat_9_1_a_schema_that_is_not_json_is_ecat_01() {
    let error = admit_catalog_schema("{\"contract\": ").expect_err("not JSON");
    assert_eq!(error.code, AdmissionCode::Ecat01);
    assert!(error.line_column.is_some());
}

#[test]
fn cat_9_1_a_state_that_is_not_json_is_wcat_01() {
    let error = admit_catalog_state("[", &admitted_schema()).expect_err("not JSON");
    assert_eq!(error.code, AdmissionCode::Wcat01);
}

#[test]
fn cat_9_1_the_contract_version_is_checked_before_the_json_schema() {
    let schema = with(schema_example(), "/contract_version", json!("0.9.0"));
    let schema = with(schema, "/relations", json!({}));
    assert_eq!(
        schema_failure(&schema),
        (AdmissionCode::Ecat02, pointer("/contract_version"))
    );
    let state = with(
        state_example(),
        "/contract",
        json!("semantic_compiler.query"),
    );
    let state = with(state, "/relations", json!({}));
    assert_eq!(
        state_failure(&state),
        (AdmissionCode::Wcat02, pointer("/contract"))
    );
}

#[test]
fn cat_9_1_the_schema_document_id_is_checked_before_the_semantic_rules() {
    // A repeated source (CAT-8.1) in a state for another schema: `WCAT-04` is reported.
    let repeated = with(
        state_example(),
        "/relations/1/source",
        json!("sales.public.fact_invoice"),
    );
    let other_schema = with(
        repeated.clone(),
        "/schema_document_id/snapshot",
        json!("schema-0043"),
    );
    assert_eq!(
        state_failure(&other_schema),
        (AdmissionCode::Wcat04, pointer("/schema_document_id"))
    );
    // The same violation in a state for the schema in use is `WCAT-03`.
    assert_eq!(
        state_failure(&repeated),
        (AdmissionCode::Wcat03, pointer("/relations/1/source"))
    );
}

#[test]
fn cat_9_state_codes_are_warnings_and_schema_codes_are_errors() {
    let warnings = [
        AdmissionCode::Wcat01,
        AdmissionCode::Wcat02,
        AdmissionCode::Wcat03,
        AdmissionCode::Wcat04,
    ];
    let errors = [
        AdmissionCode::Ecat01,
        AdmissionCode::Ecat02,
        AdmissionCode::Ecat03,
    ];
    assert!(warnings.iter().all(|code| code.is_warning()));
    assert!(errors.iter().all(|code| !code.is_warning()));
    let texts: Vec<&str> = warnings
        .iter()
        .chain(errors.iter())
        .map(|code| code.as_str())
        .collect();
    assert_eq!(
        texts,
        [
            "WCAT-01", "WCAT-02", "WCAT-03", "WCAT-04", "ECAT-01", "ECAT-02", "ECAT-03"
        ]
    );
}

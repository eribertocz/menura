//! The conditions and time fields of populations in E0.f: POP-4.4, POP-4.5, POP-4.6, `E0C-23`
//! and `E0C-24`.

use menura_contracts::admit_catalog_schema;
use menura_contracts::types::audit_record::OssieFormat;

use crate::extensions::populations::fixture::{
    ACCOUNTS_DATA, CUSTOMERS_DATA, MODEL, PERIODIC, VALIDITY, conformance_model, with_populations,
    with_validity,
};
use crate::extensions::summaries::SummaryDatasets;
use crate::extensions::{DEFAULT_NAMESPACE, validate_extensions};
use crate::read::read_document;
use crate::references::derived::DerivedMetrics;
use crate::structure::validate_structure;
use crate::typing::populations::type_populations;
use crate::typing::type_expressions;

/// The catalog schema of every `e0-populations*` model: `shop.customers`
/// (`customer_id` 32-bit, `segment` text, `active` boolean, `valid_from` date, nullable
/// `valid_to` date) and `shop.account_months` (`account_id` 32-bit, `period_start` date,
/// `balance` 32-bit).
const CATALOG: &str =
    include_str!("../../../../../conformance/models/e0-populations/catalog-schema.json");

/// The code and JSON Pointer of every diagnostic of the populations of `model`.
fn problems(model: &str) -> Vec<(&'static str, String)> {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure =
        validate_structure(&document, OssieFormat::Release, None).expect("the structure is valid");
    let catalog = admit_catalog_schema(CATALOG).expect("the catalog schema is admitted");
    let extensions = validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
        .expect("the extensions are accepted");
    let typing = type_expressions(
        &document,
        &structure,
        &catalog,
        &DerivedMetrics::default(),
        &SummaryDatasets::default(),
    );
    match type_populations(&structure, &typing, &extensions) {
        Ok(()) => Vec::new(),
        Err(errors) => errors
            .iter()
            .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
            .collect(),
    }
}

fn customers(code: &'static str) -> Vec<(&'static str, String)> {
    vec![(code, CUSTOMERS_DATA.to_owned())]
}

/// What the population of `customers` with the condition `condition` finds.
fn condition(condition: &str) -> Vec<(&'static str, String)> {
    let text = serde_json::to_string(condition).expect("a JSON string");
    problems(&with_validity(&VALIDITY.replace(r#""active""#, &text)))
}

#[test]
fn pop_4_4_pop_4_5_valid_populations_have_no_diagnostic() {
    assert!(problems(MODEL).is_empty());
}

#[test]
fn pop_4_4_pop_4_6_e0c_23_a_condition_that_is_not_boolean() {
    assert_eq!(
        problems(conformance_model("e0-populations-invalid-condition-001")),
        customers("E0C-23")
    );
}

#[test]
fn pop_4_4_boolean_row_level_conditions_are_valid() {
    for valid in [
        "active",
        "segment = 'RETAIL' AND valid_to IS NULL",
        "CASE WHEN active THEN valid_from > DATE '2026-01-01' ELSE FALSE END",
        "\"ACTIVE\" OR customer_id IN (1, 2)",
    ] {
        assert!(condition(valid).is_empty(), "{valid}");
    }
}

#[test]
fn pop_4_4_e0c_23_every_invalid_condition() {
    for invalid in [
        "active AND",
        "RANDOM() > 0.5",
        "COUNT(*) OVER () > 1",
        "customers.active",
        "COUNT(*) > 0",
        "AGG(customer_count) > 0",
        "USER_ATTRIBUTE('segment') = segment",
        "NULL",
        "customer_id + 1",
        "segment = 1",
    ] {
        assert_eq!(condition(invalid), customers("E0C-23"), "{invalid}");
    }
}

#[test]
fn pop_4_5_pop_4_6_e0c_24_a_time_field_that_is_not_a_date_or_timestamp() {
    assert_eq!(
        problems(conformance_model("e0-populations-period-kind-001")),
        [("E0C-24", ACCOUNTS_DATA.to_owned())]
    );
    let valid_to = VALIDITY.replace(r#""valid_to": "valid_to""#, r#""valid_to": "segment""#);
    assert_eq!(problems(&with_validity(&valid_to)), customers("E0C-24"));
}

#[test]
fn pop_4_5_a_timestamp_field_is_accepted() {
    let model = with_validity(VALIDITY).replace(
        "expression: {dialects: [{dialect: ANSI_SQL, expression: valid_to}]}",
        "expression: {dialects: [{dialect: ANSI_SQL, expression: 'CAST(valid_to AS TIMESTAMP)'}]}",
    );
    assert!(problems(&model).is_empty());
}

#[test]
fn pop_4_5_fields_without_a_type_are_not_checked() {
    let untyped = |field: &str| {
        let from =
            format!("expression: {{dialects: [{{dialect: ANSI_SQL, expression: {field}}}]}}");
        let to = format!(
            "expression: {{dialects: [{{dialect: ANSI_SQL, expression: '{field} + segment'}}]}}"
        );
        MODEL.replacen(&from, &to, 1)
    };
    assert!(problems(&untyped("valid_from")).is_empty());
    assert!(problems(&untyped("active")).is_empty());
}

/// [`MODEL`] whose field `active` has no type and whose population condition is `condition`.
fn untyped_active(condition: &str) -> Vec<(&'static str, String)> {
    let text = serde_json::to_string(condition).expect("a JSON string");
    let model = with_validity(&VALIDITY.replace(r#""active""#, &text)).replacen(
        "expression: {dialects: [{dialect: ANSI_SQL, expression: active}]}",
        "expression: {dialects: [{dialect: ANSI_SQL, expression: 'active + segment'}]}",
        1,
    );
    problems(&model)
}

#[test]
fn pop_4_5_e0c_23_structural_conditions_are_checked_whatever_the_field_types() {
    assert_eq!(
        problems(conformance_model(
            "e0-populations-untyped-field-condition-001"
        )),
        customers("E0C-23")
    );
    for invalid in [
        "active AND",
        "active AND RANDOM() > 0.5",
        "active AND customers.segment = 'RETAIL'",
        "active AND COUNT(*) > 0",
        "active AND AGG(customer_count) > 0",
        "active AND USER_ATTRIBUTE('segment') = segment",
    ] {
        assert_eq!(untyped_active(invalid), customers("E0C-23"), "{invalid}");
    }
}

#[test]
fn pop_4_5_the_type_is_not_checked_when_a_field_read_has_no_type() {
    for unchecked in ["active", "active + 1", "segment = 1 OR active"] {
        assert!(untyped_active(unchecked).is_empty(), "{unchecked}");
    }
}

#[test]
fn pop_4_6_diagnostics_are_in_document_order_then_by_code() {
    let validity = VALIDITY
        .replace(
            r#""valid_from": "valid_from""#,
            r#""valid_from": "customer_id""#,
        )
        .replace(r#""active""#, r#""segment""#);
    let period = PERIODIC.replace("period_start", "balance");
    let model = with_populations(&format!("[{validity}]"), &format!("[{period}]"));
    assert_eq!(
        problems(&model),
        [
            ("E0C-23", CUSTOMERS_DATA.to_owned()),
            ("E0C-24", CUSTOMERS_DATA.to_owned()),
            ("E0C-24", ACCOUNTS_DATA.to_owned()),
        ]
    );
}

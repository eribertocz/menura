//! The conditions of row policies in E0.f: SCY-4.1, SCY-4.2, SCY-5.1.6, `E0C-46` and
//! `E0C-48`.

use semantic_compiler_contracts::admit_catalog_schema;
use semantic_compiler_contracts::types::audit_record::OssieFormat;

use crate::extensions::security::fixture::{
    CATALOG, MODEL, NOTES_DATA, ORDERS_DATA, conformance_model, with, with_notes, with_row_access,
};
use crate::extensions::summaries::SummaryDatasets;
use crate::extensions::{DEFAULT_NAMESPACE, validate_extensions};
use crate::read::read_document;
use crate::references::derived::DerivedMetrics;
use crate::structure::validate_structure;
use crate::typing::security::type_row_policies;
use crate::typing::type_expressions;

/// The code and JSON Pointer of every diagnostic of the row policies of `model`.
fn problems_in(model: &str, format: OssieFormat) -> Vec<(&'static str, String)> {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure = validate_structure(&document, format, None).expect("the structure is valid");
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
    match type_row_policies(&structure, &typing, &extensions) {
        Ok(()) => Vec::new(),
        Err(errors) => errors
            .iter()
            .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
            .collect(),
    }
}

fn problems(model: &str) -> Vec<(&'static str, String)> {
    problems_in(model, OssieFormat::Release)
}

/// [`MODEL`] where `orders` has the single row policy `condition`.
fn with_condition(condition: &str) -> String {
    let condition = condition.replace('\'', "''");
    with_row_access(&format!(r#"[{{"name": "p", "condition": "{condition}"}}]"#))
}

/// The codes of the problems of `condition` as the single row policy of `orders`.
fn codes(condition: &str) -> Vec<&'static str> {
    problems(&with_condition(condition))
        .into_iter()
        .map(|(code, pointer)| {
            assert_eq!(pointer, ORDERS_DATA, "{condition}");
            code
        })
        .collect()
}

#[test]
fn scy_5_1_6_the_conditions_of_the_valid_model_are_boolean() {
    assert!(problems(MODEL).is_empty());
}

#[test]
fn scy_4_1_a_list_attribute_as_the_single_item_of_in_or_not_in() {
    for condition in [
        "region IN (USER_ATTRIBUTE('regions'))",
        "region NOT IN (USER_ATTRIBUTE('regions'))",
        "amount > 10 AND region IN (USER_ATTRIBUTE('regions'))",
    ] {
        assert!(codes(condition).is_empty(), "{condition}");
    }
}

#[test]
fn scy_4_1_scy_5_1_6_e0c_48_a_list_attribute_anywhere_else() {
    for condition in [
        "region = USER_ATTRIBUTE('regions')",
        "region IN (USER_ATTRIBUTE('regions'), 'NORTH')",
        "USER_ATTRIBUTE('regions') IN ('NORTH')",
        "USER_ATTRIBUTE('regions') IS NULL",
        "region IN (UPPER(USER_ATTRIBUTE('regions')))",
    ] {
        assert_eq!(codes(condition), ["E0C-48"], "{condition}");
    }
    assert_eq!(
        problems(conformance_model("e0-security-list-attribute-001")),
        [("E0C-48", ORDERS_DATA.to_owned())]
    );
}

#[test]
fn scy_5_1_6_scalar_attributes_have_their_declared_type() {
    for condition in [
        "amount = USER_ATTRIBUTE('area_id')",
        "amount <= USER_ATTRIBUTE('limit')",
        "region IN (USER_ATTRIBUTE('regions')) OR amount < USER_ATTRIBUTE('limit')",
    ] {
        assert!(codes(condition).is_empty(), "{condition}");
    }
    assert_eq!(codes("region = USER_ATTRIBUTE('area_id')"), ["E0C-46"]);
    assert_eq!(codes("region IN (USER_ATTRIBUTE('area_id'))"), ["E0C-46"]);
    assert_eq!(codes("amount IN (USER_ATTRIBUTE('regions'))"), ["E0C-46"]);
}

#[test]
fn scy_4_2_scy_5_1_6_e0c_46_constructs_a_condition_may_not_hold() {
    for condition in [
        "region IN",
        "orders.region = 'NORTH'",
        "zone = 'NORTH'",
        "SUM(amount) > 100",
        "AGG(total_amount) > 100",
        "region = USER_ATTRIBUTE('team')",
        "amount",
        "NULL",
        "region + 1 = 2",
    ] {
        assert_eq!(codes(condition), ["E0C-46"], "{condition}");
    }
    for name in [
        "e0-security-undeclared-attribute-001",
        "e0-security-aggregate-condition-001",
        "e0-security-not-boolean-condition-001",
    ] {
        assert_eq!(
            problems(conformance_model(name)),
            [("E0C-46", ORDERS_DATA.to_owned())],
            "{name}"
        );
    }
}

#[test]
fn scy_5_1_6_e0c_46_and_e0c_48_are_both_reported() {
    let row_access = r#"[{"name": "a", "condition": "region = USER_ATTRIBUTE(''regions'')"}, {"name": "b", "condition": "zone = 1"}]"#;
    assert_eq!(
        problems(&with_row_access(row_access)),
        [
            ("E0C-46", ORDERS_DATA.to_owned()),
            ("E0C-48", ORDERS_DATA.to_owned())
        ]
    );
}

#[test]
fn scy_5_1_6_the_type_is_checked_only_when_every_field_read_has_one() {
    let untyped = with(
        "          - name: cost_copy\n            expression: {dialects: [{dialect: ANSI_SQL, expression: 'unit_cost'}]}",
        "          - name: cost_copy\n            expression: {dialects: [{dialect: ANSI_SQL, expression: 'no_such_column'}]}",
    );
    let condition = |text: &str| {
        untyped.replace(
            r#""condition": "TRUE""#,
            &format!(r#""condition": "{}""#, text.replace('\'', "''")),
        )
    };
    assert!(problems(&condition("cost_copy")).is_empty());
    assert_eq!(
        problems(&condition("SUM(cost_copy) > 1")),
        [("E0C-46", ORDERS_DATA.to_owned())]
    );
    assert_eq!(
        problems(&condition("cost_copy = USER_ATTRIBUTE('regions')")),
        [("E0C-48", ORDERS_DATA.to_owned())]
    );
}

#[test]
fn scy_5_1_6_one_diagnostic_per_entry() {
    let model = with_notes(
        r#""access": {"mode": "authenticated"}, "row_access": [{"name": "n1", "condition": "body"}, {"name": "n2", "condition": "note_id"}]"#,
    );
    let model = model.replace(r#""condition": "TRUE""#, r#""condition": "1""#);
    assert_eq!(
        problems(&model),
        [
            ("E0C-46", ORDERS_DATA.to_owned()),
            ("E0C-46", NOTES_DATA.to_owned())
        ]
    );
}

#[test]
fn mdl_2_4_scy_5_1_6_the_development_conditions_are_typed() {
    assert!(
        problems_in(
            conformance_model("e0-security-development"),
            OssieFormat::Development
        )
        .is_empty()
    );
}

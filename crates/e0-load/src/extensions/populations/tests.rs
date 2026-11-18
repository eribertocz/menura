//! The populations extension in E0.c: POP-2, POP-4.1 to POP-4.3, POP-4.6, `E0C-02`, `E0C-03`,
//! `E0C-20` and `E0C-21`.
//!
//! The model documents of the `e0-populations*` conformance cases are read here at compile
//! time (see [`super::fixture`]), so the rules are checked against the files the conformance
//! suite uses.

use semantic_compiler_contracts::types::audit_record::OssieFormat;
use semantic_compiler_contracts::types::common::Grain;

use super::model::{PartialPeriod, Population, PopulationMode};
use crate::extensions::output::{Extensions, ExtensionsFailure};
use crate::extensions::{DEFAULT_NAMESPACE, validate_extensions};
use crate::read::read_document;
use crate::structure::validate_structure;

use super::fixture::{
    ACCOUNTS_DATA, CUSTOMERS, CUSTOMERS_DATA, MODEL, PERIODIC, VALIDITY, conformance_model,
    with_populations, with_validity,
};

fn validate(model: &str) -> Result<Extensions, ExtensionsFailure> {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure =
        validate_structure(&document, OssieFormat::Release, None).expect("the structure is valid");
    validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
}

/// The code and JSON Pointer of every rejecting diagnostic.
fn errors(model: &str) -> Vec<(&'static str, String)> {
    validate(model)
        .expect_err("the extensions are rejected")
        .errors
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect()
}

fn messages(model: &str) -> Vec<String> {
    validate(model)
        .expect_err("the extensions are rejected")
        .errors
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect()
}

fn pair(code: &'static str, pointer: &str) -> (&'static str, String) {
    (code, pointer.to_owned())
}

#[test]
fn pop_2_pop_4_1_pop_4_2_valid_populations_are_read() {
    let extensions = validate(MODEL).expect("the extensions are accepted");
    let customers = &extensions.populations[0];
    assert_eq!(
        customers,
        &Population {
            dataset: customers.dataset.clone(),
            data: customers.data.clone(),
            location: customers.location,
            name: "active_customers".to_owned(),
            entity: vec!["customer_id".to_owned()],
            mode: PopulationMode::Validity {
                valid_from: "valid_from".to_owned(),
                valid_to: "valid_to".to_owned(),
                partial_period: PartialPeriod::AnyDay,
            },
            required_when: Some("active".to_owned()),
        }
    );
    assert_eq!(customers.dataset.as_str(), CUSTOMERS);
    assert_eq!(customers.data.as_str(), CUSTOMERS_DATA);
    let accounts = &extensions.populations[1];
    assert_eq!(accounts.data.as_str(), ACCOUNTS_DATA);
    assert_eq!(
        accounts.mode,
        PopulationMode::Periodic {
            period: "period_start".to_owned(),
            grain: Grain::Month,
        }
    );
    assert_eq!(accounts.required_when, None);
    assert_eq!(extensions.populations.len(), 2);
    assert!(extensions.warnings.is_empty());
}

#[test]
fn pop_4_2_every_value_of_each_set_is_accepted() {
    for partial_period in ["any_day", "full_period", "period_end"] {
        let item = VALIDITY.replace("any_day", partial_period);
        assert!(validate(&with_validity(&item)).is_ok(), "{partial_period}");
    }
    for grain in ["day", "week", "month", "quarter", "year"] {
        let item = PERIODIC.replace(r#""grain": "month""#, &format!(r#""grain": "{grain}""#));
        let model = with_populations(&format!("[{VALIDITY}]"), &format!("[{item}]"));
        assert!(validate(&model).is_ok(), "{grain}");
    }
    let empty = with_populations("[]", "[]");
    assert!(validate(&empty).expect("accepted").populations.is_empty());
}

#[test]
fn pop_4_3_pop_4_6_e0c_20_duplicate_name_at_the_later_entry() {
    assert_eq!(
        errors(conformance_model("e0-populations-duplicate-name-001")),
        [pair("E0C-20", ACCOUNTS_DATA)]
    );
    let upper = VALIDITY.replace("active_customers", "ACTIVE_CUSTOMERS");
    let same_entry = with_populations(&format!("[{VALIDITY}, {upper}]"), &format!("[{PERIODIC}]"));
    assert_eq!(errors(&same_entry), [pair("E0C-20", CUSTOMERS_DATA)]);
    let delimited = VALIDITY.replace("active_customers", r#"\"active_customers\""#);
    let model = with_populations(
        &format!("[{VALIDITY}, {delimited}]"),
        &format!("[{PERIODIC}]"),
    );
    assert!(
        validate(&model).is_ok(),
        "a delimited lower-case name is another name"
    );
}

#[test]
fn pop_4_1_e0c_21_missing_key_or_key_of_the_other_mode() {
    assert_eq!(
        errors(conformance_model("e0-populations-mode-keys-001")),
        [pair("E0C-21", CUSTOMERS_DATA)]
    );
    for key in ["name", "mode", "entity"] {
        let item = VALIDITY.replacen(&format!(r#""{key}": "#), r#""ignored": "#, 1);
        let codes = errors(&with_validity(&item));
        assert!(codes.contains(&pair("E0C-21", CUSTOMERS_DATA)), "{key}");
    }
    let periodic_without_grain = PERIODIC.replace(r#", "grain": "month""#, "");
    let model = with_populations(
        &format!("[{VALIDITY}]"),
        &format!("[{periodic_without_grain}]"),
    );
    assert_eq!(errors(&model), [pair("E0C-21", ACCOUNTS_DATA)]);
    let periodic_with_validity_key = PERIODIC.replace('}', r#", "valid_to": "period_start"}"#);
    let model = with_populations(
        &format!("[{VALIDITY}]"),
        &format!("[{periodic_with_validity_key}]"),
    );
    assert_eq!(errors(&model), [pair("E0C-21", ACCOUNTS_DATA)]);
}

#[test]
fn pop_4_1_e0c_2_4_e0c_02_unknown_key_inside_an_item() {
    assert_eq!(
        errors(conformance_model("e0-populations-unknown-key-001")),
        [pair("E0C-02", ACCOUNTS_DATA)]
    );
}

#[test]
fn pop_4_2_e0c_03_wrong_type_or_value_outside_its_set() {
    assert_eq!(
        errors(conformance_model("e0-populations-wrong-value-001")),
        [pair("E0C-03", CUSTOMERS_DATA)]
    );
    let wrong = [
        VALIDITY.replace(r#""active_customers""#, r#""""#),
        VALIDITY.replace(r#""active_customers""#, "5"),
        VALIDITY.replace(r#""validity""#, r#""intervals""#),
        VALIDITY.replace(r#"["customer_id"]"#, "[]"),
        VALIDITY.replace(r#"["customer_id"]"#, r#""customer_id""#),
        VALIDITY.replace(r#"["customer_id"]"#, r#"["customer_id", 1]"#),
        VALIDITY.replace(r#"["customer_id"]"#, r#"["customer_id", "CUSTOMER_ID"]"#),
        VALIDITY.replace(r#""valid_from": "valid_from""#, r#""valid_from": null"#),
        VALIDITY.replace(r#""required_when": "active""#, r#""required_when": true"#),
    ];
    for item in wrong {
        assert_eq!(
            errors(&with_validity(&item)),
            [pair("E0C-03", CUSTOMERS_DATA)],
            "{item}"
        );
    }
    let grain = PERIODIC.replace(r#""grain": "month""#, r#""grain": "MONTH""#);
    let model = with_populations(&format!("[{VALIDITY}]"), &format!("[{grain}]"));
    assert_eq!(errors(&model), [pair("E0C-03", ACCOUNTS_DATA)]);
    let not_a_list = with_populations(VALIDITY, &format!("[{PERIODIC}]"));
    assert_eq!(errors(&not_a_list), [pair("E0C-03", CUSTOMERS_DATA)]);
    let not_an_object = with_populations(r#"["active_customers"]"#, &format!("[{PERIODIC}]"));
    assert_eq!(errors(&not_an_object), [pair("E0C-03", CUSTOMERS_DATA)]);
}

#[test]
fn pop_4_6_one_diagnostic_per_code_and_entry_sorted_by_code() {
    let first = VALIDITY
        .replace(r#""any_day""#, r#""never""#)
        .replace('}', r#", "owner": 1}"#);
    let second = VALIDITY
        .replace(r#", "partial_period": "any_day""#, "")
        .replace(r#""valid_to": "valid_to""#, r#""valid_to": 3"#);
    let model = with_populations(&format!("[{first}, {second}]"), &format!("[{PERIODIC}]"));
    assert_eq!(
        errors(&model),
        [
            pair("E0C-02", CUSTOMERS_DATA),
            pair("E0C-03", CUSTOMERS_DATA),
            pair("E0C-20", CUSTOMERS_DATA),
            pair("E0C-21", CUSTOMERS_DATA),
        ]
    );
    let found = messages(&model);
    assert!(
        found[1].contains("item 0") && found[1].contains("item 1"),
        "{}",
        found[1]
    );
}

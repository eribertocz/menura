//! The temporal extension in E0.c: VAL-2.1, `E0C-02`, `E0C-03` and `E0C-05`.
//!
//! The model documents of the `val-2-1-*` conformance cases are read here at compile time, so
//! the rules are checked against the files the conformance suite uses.

use menura_contracts::types::audit_record::OssieFormat;

use crate::extensions::output::{Extensions, ExtensionsFailure};
use crate::extensions::{DEFAULT_NAMESPACE, validate_extensions};
use crate::read::read_document;
use crate::structure::validate_structure;

const MODEL: &str = include_str!("../../../../../conformance/models/val-temporal/model.yaml");

/// The `data` member of the entry of the first relationship.
const DATA: &str = "/semantic_model/0/relationships/0/custom_extensions/0/data";

fn validate(model: &str) -> Result<Extensions, ExtensionsFailure> {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure =
        validate_structure(&document, OssieFormat::Release, None).expect("the structure is valid");
    validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
}

/// The code and JSON Pointer of every rejecting diagnostic of `model`.
fn rejected(model: &str) -> Vec<(&'static str, String)> {
    validate(model)
        .expect_err("the extensions are rejected")
        .errors
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect()
}

fn at_data(code: &'static str) -> Vec<(&'static str, String)> {
    vec![(code, DATA.to_owned())]
}

#[test]
fn val_2_1_the_declarations_of_the_model_are_read() {
    let extensions = validate(MODEL).expect("the extensions are accepted");
    let found: Vec<(String, &str, &str)> = extensions
        .temporal
        .iter()
        .map(|declaration| {
            (
                declaration.relationship.to_string(),
                declaration.from_time.as_str(),
                declaration.population.as_str(),
            )
        })
        .collect();
    assert_eq!(
        found,
        [
            (
                "/semantic_model/0/relationships/0".to_owned(),
                "sold_on",
                "customer_history"
            ),
            (
                "/semantic_model/0/relationships/2".to_owned(),
                "visited_at",
                "tier_history"
            ),
        ]
    );
}

#[test]
fn val_2_1_e0c_02_an_unknown_key() {
    let model =
        include_str!("../../../../../conformance/models/val-2-1-unknown-key-001/model.yaml");
    assert_eq!(rejected(model), at_data("E0C-02"));
}

#[test]
fn val_2_1_e0c_03_a_missing_key() {
    let model =
        include_str!("../../../../../conformance/models/val-2-1-missing-key-001/model.yaml");
    assert_eq!(rejected(model), at_data("E0C-03"));
}

#[test]
fn val_2_1_e0c_03_an_empty_name() {
    let model = include_str!("../../../../../conformance/models/val-2-1-empty-name-001/model.yaml");
    assert_eq!(rejected(model), at_data("E0C-03"));
}

#[test]
fn val_2_1_e0c_03_a_value_that_is_not_an_object_or_not_a_string() {
    let not_object = MODEL.replacen(
        r#""temporal": {"from_time": "sold_on", "population": "customer_history"}"#,
        r#""temporal": ["sold_on"]"#,
        1,
    );
    assert_eq!(rejected(&not_object), at_data("E0C-03"));
    let not_string = MODEL.replacen(r#""from_time": "sold_on""#, r#""from_time": 7"#, 1);
    assert_eq!(rejected(&not_string), at_data("E0C-03"));
}

#[test]
fn val_2_1_e0c_05_temporal_is_valid_on_relationships_only() {
    let on_dataset = MODEL.replacen(
        r#""access": {"mode": "roles", "roles": ["analyst"]}}"#,
        r#""access": {"mode": "roles", "roles": ["analyst"]}, "temporal": {"from_time": "sold_on", "population": "customer_history"}}"#,
        1,
    );
    let codes: Vec<&str> = rejected(&on_dataset)
        .into_iter()
        .map(|(code, _)| code)
        .collect();
    assert_eq!(codes, ["E0C-05"]);
}

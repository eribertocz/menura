//! Admission of the security context: SEC-3.8 order of checks and SEC-3.3 declared types,
//! with the ignored roles and attributes of SEC-3.2 and SEC-3.2a.

mod support;

use menura_contracts::AdmissionCode;
use menura_contracts::types::compiled_plan::InputDocument;
use menura_model::internal_model::{AttributeType, ScalarType, SecurityContext};
use menura_security::{
    AdmittedScalar, AdmittedValue, DateValue, DecimalValue, Role, admit_security_context,
};
use support::{decimal, declarations, document, list, scalar};

/// The code and pointer of the first failure.
// `cfg(test)` marks the helper as test code, where `unwrap` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
fn failure(text: &str, model: Option<&SecurityContext>) -> (AdmissionCode, String) {
    let error = admit_security_context(text, model).unwrap_err();
    assert_eq!(error.document, InputDocument::SecurityContext);
    (error.code, error.pointer)
}

fn esec_01(pointer: &str) -> (AdmissionCode, String) {
    (AdmissionCode::Esec01, pointer.to_owned())
}

/// Admits one attribute `a` of `attribute_type` with the JSON `value`.
fn admit_one(
    attribute_type: AttributeType,
    value: &str,
) -> Result<AdmittedValue, (AdmissionCode, String)> {
    let model = declarations(&[], &[("a", attribute_type)]);
    let text = document("[]", &format!(r#"{{"a": {value}}}"#));
    admit_security_context(&text, Some(&model))
        .map(|context| context.attributes()["a"].clone())
        .map_err(|error| (error.code, error.pointer))
}

// `cfg(test)` marks the helper as test code, where `unwrap` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
fn accepted(attribute_type: AttributeType, value: &str) -> AdmittedValue {
    admit_one(attribute_type, value).unwrap_or_else(|error| panic!("{value}: {error:?}"))
}

fn rejected_at(attribute_type: AttributeType, value: &str, pointer: &str) {
    assert_eq!(
        admit_one(attribute_type, value),
        Err(esec_01(pointer)),
        "{value}"
    );
}

fn one(value: AdmittedScalar) -> AdmittedValue {
    AdmittedValue::Scalar(value)
}

#[test]
fn sec_3_8_not_json_is_esec_01_at_the_empty_pointer_with_line_and_column() {
    let error = admit_security_context("{\n  \"contract\": ", None).unwrap_err();
    assert_eq!(error.code, AdmissionCode::Esec01);
    assert_eq!(error.pointer, "");
    assert_eq!(error.line_column, Some((2, 14)));
    let location = error.diagnostic().location.unwrap();
    assert_eq!(location.document, InputDocument::SecurityContext);
    assert_eq!((location.line, location.column), (Some(2), Some(14)));
}

#[test]
fn sec_3_8_missing_contract_is_esec_03_at_contract() {
    let text = r#"{"contract_version": "1.1.0", "subject": "u", "roles": [], "attributes": {}}"#;
    assert_eq!(
        failure(text, None),
        (AdmissionCode::Esec03, "/contract".to_owned())
    );
}

#[test]
fn sec_3_8_unsupported_major_version_is_esec_03_at_contract_version() {
    let text = document("[]", "{}").replace("1.1.0", "2.0.0");
    assert_eq!(
        failure(&text, None),
        (AdmissionCode::Esec03, "/contract_version".to_owned())
    );
}

#[test]
fn sec_3_8_contract_is_checked_before_the_schema() {
    let text = r#"{"contract": "menura.query", "contract_version": "1.1.0", "extra": 1}"#;
    assert_eq!(
        failure(text, None),
        (AdmissionCode::Esec03, "/contract".to_owned())
    );
}

#[test]
fn sec_3_8_schema_failure_is_esec_01() {
    let text = document("[]", "{}").replace(r#""subject": "user-1","#, "");
    assert_eq!(failure(&text, None).0, AdmissionCode::Esec01);
    let text = document("[1]", "{}");
    assert_eq!(failure(&text, None), esec_01("/roles/0"));
}

#[test]
fn sec_3_8_the_schema_is_checked_before_the_declarations() {
    let model = declarations(&[], &[("a", scalar(ScalarType::Integer))]);
    let text = document(r#"["x", "x"]"#, r#"{"a": "wrong"}"#);
    assert_eq!(failure(&text, Some(&model)), esec_01("/roles/1"));
}

#[test]
fn sec_3_8_the_first_wrong_attribute_in_name_order_is_reported() {
    let model = declarations(
        &[],
        &[
            ("b", scalar(ScalarType::Integer)),
            ("a", scalar(ScalarType::Boolean)),
        ],
    );
    let text = document("[]", r#"{"b": "x", "a": 1}"#);
    assert_eq!(failure(&text, Some(&model)), esec_01("/attributes/a"));
}

#[test]
fn sec_3_8_list_items_are_checked_in_index_order() {
    let model = declarations(&[], &[("a", list(ScalarType::Integer))]);
    let text = document("[]", r#"{"a": [1, "x", true]}"#);
    assert_eq!(failure(&text, Some(&model)), esec_01("/attributes/a/1"));
}

#[test]
fn sec_3_8_attribute_names_are_escaped_in_the_pointer() {
    let model = declarations(&[], &[("a/b~c", scalar(ScalarType::Integer))]);
    let text = document("[]", r#"{"a/b~c": "x"}"#);
    assert_eq!(failure(&text, Some(&model)), esec_01("/attributes/a~1b~0c"));
}

#[test]
fn sec_3_3_text() {
    assert_eq!(
        accepted(scalar(ScalarType::Text), r#""north""#),
        one(AdmittedScalar::Text("north".to_owned()))
    );
    rejected_at(scalar(ScalarType::Text), "1", "/attributes/a");
    rejected_at(scalar(ScalarType::Text), "true", "/attributes/a");
}

#[test]
fn sec_3_3_integer() {
    let integer = scalar(ScalarType::Integer);
    assert_eq!(accepted(integer, "-42"), one(AdmittedScalar::Integer(-42)));
    assert_eq!(
        accepted(integer, "9007199254740991"),
        one(AdmittedScalar::Integer(9_007_199_254_740_991))
    );
    assert_eq!(
        accepted(integer, r#""9007199254740992""#),
        one(AdmittedScalar::Integer(9_007_199_254_740_992))
    );
    assert_eq!(
        accepted(integer, r#""-9223372036854775808""#),
        one(AdmittedScalar::Integer(i64::MIN))
    );
    for value in [
        r#""42""#,
        r#""9007199254740991""#,
        r#""09007199254740992""#,
        r#""+9007199254740992""#,
        r#""9223372036854775808""#,
        r#""1e20""#,
        "true",
    ] {
        rejected_at(integer, value, "/attributes/a");
    }
}

#[test]
fn sec_3_3_decimal() {
    let money = scalar(decimal(5, 2));
    let value = |unscaled, scale| one(AdmittedScalar::Decimal(DecimalValue::new(unscaled, scale)));
    assert_eq!(accepted(money, r#""-123.45""#), value(-12_345, 2));
    assert_eq!(accepted(money, r#""0.00""#), value(0, 2));
    assert_eq!(accepted(scalar(decimal(2, 2)), r#""0.50""#), value(50, 2));
    assert_eq!(accepted(scalar(decimal(3, 0)), r#""999""#), value(999, 0));
    for value in [
        r#""1.5""#,
        r#""1.500""#,
        r#""1234.00""#,
        r#""-0.00""#,
        r#""01.00""#,
        r#""1""#,
        "1",
    ] {
        rejected_at(money, value, "/attributes/a");
    }
    rejected_at(scalar(decimal(3, 0)), r#""1.0""#, "/attributes/a");
}

#[test]
fn sec_3_3_date() {
    let date = scalar(ScalarType::Date);
    assert_eq!(
        accepted(date, r#""2024-02-29""#),
        one(AdmittedScalar::Date(DateValue::new(2024, 2, 29).unwrap()))
    );
    for value in [
        r#""2026-02-30""#,
        r#""2023-02-29""#,
        r#""2026-2-01""#,
        r#""2026-01-01T00:00:00""#,
        "20260101",
    ] {
        rejected_at(date, value, "/attributes/a");
    }
}

#[test]
fn sec_3_3_boolean() {
    let boolean = scalar(ScalarType::Boolean);
    assert_eq!(
        accepted(boolean, "false"),
        one(AdmittedScalar::Boolean(false))
    );
    rejected_at(boolean, r#""true""#, "/attributes/a");
    rejected_at(boolean, "1", "/attributes/a");
}

#[test]
fn sec_3_3_list() {
    let texts = list(ScalarType::Text);
    assert_eq!(accepted(texts, "[]"), AdmittedValue::List(Vec::new()));
    assert_eq!(
        accepted(texts, r#"["b", "a"]"#),
        AdmittedValue::List(vec![
            AdmittedScalar::Text("b".to_owned()),
            AdmittedScalar::Text("a".to_owned()),
        ])
    );
    rejected_at(texts, r#""a""#, "/attributes/a");
    rejected_at(texts, r#"["a", 1]"#, "/attributes/a/1");
    rejected_at(
        list(ScalarType::Date),
        r#"["2026-02-30"]"#,
        "/attributes/a/0",
    );
}

#[test]
fn sec_3_3_an_array_is_never_accepted_for_a_scalar() {
    rejected_at(scalar(ScalarType::Text), r#"["a"]"#, "/attributes/a");
}

#[test]
fn sec_3_2_undeclared_roles_grant_nothing_and_are_ignored() {
    let model = declarations(&["analyst", "admin"], &[]);
    let text = document(r#"["viewer", "analyst", "Admin"]"#, "{}");
    let context = admit_security_context(&text, Some(&model)).unwrap();
    assert_eq!(context.roles(), [Role::new("analyst")]);
    assert_eq!(
        context.ignored_roles(),
        [Role::new("Admin"), Role::new("viewer")]
    );
}

#[test]
fn sec_3_2a_undeclared_attributes_are_ignored_and_not_checked() {
    let model = declarations(&[], &[("area", scalar(ScalarType::Integer))]);
    let text = document("[]", r#"{"zone": [1, "x"], "area": 7, "other": true}"#);
    let context = admit_security_context(&text, Some(&model)).unwrap();
    assert_eq!(context.attributes().len(), 1);
    assert_eq!(
        context.attributes()["area"],
        one(AdmittedScalar::Integer(7))
    );
    assert_eq!(context.ignored_attributes(), ["other", "zone"]);
}

#[test]
fn sec_3_2_a_model_without_security_context_declares_nothing() {
    let text = document(r#"["analyst"]"#, r#"{"a": 1}"#);
    let context = admit_security_context(&text, None).unwrap();
    assert!(context.roles().is_empty());
    assert!(context.attributes().is_empty());
    assert_eq!(context.ignored_roles(), [Role::new("analyst")]);
    assert_eq!(context.ignored_attributes(), ["a"]);
    assert_eq!(context.subject(), "user-1");
}

#[test]
fn e2_5_row_0a_order_not_json_then_contract_then_schema_then_declarations() {
    let model = declarations(&[], &[("a", scalar(ScalarType::Integer))]);
    assert_eq!(failure("[", Some(&model)), esec_01(""));
    assert_eq!(
        failure(r#"{"a": "x"}"#, Some(&model)).0,
        AdmissionCode::Esec03
    );
    let text = document("[]", r#"{"a": "x"}"#);
    assert_eq!(failure(&text, Some(&model)), esec_01("/attributes/a"));
}

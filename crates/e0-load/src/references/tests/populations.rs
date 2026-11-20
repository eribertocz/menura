//! The fields of populations in E0.d: POP-4.3, POP-4.6 and `E0C-22`.

use menura_contracts::types::audit_record::OssieFormat;

use crate::extensions::populations::fixture::{
    ACCOUNTS_DATA, CUSTOMERS_DATA, MODEL, PERIODIC, VALIDITY, conformance_model, with_populations,
    with_validity,
};
use crate::extensions::{DEFAULT_NAMESPACE, validate_extensions};
use crate::read::read_document;
use crate::references::populations::resolve_population_references;
use crate::structure::validate_structure;

/// The code and JSON Pointer of every `E0C-22` diagnostic of `model`.
fn missing(model: &str) -> Vec<(&'static str, String)> {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure =
        validate_structure(&document, OssieFormat::Release, None).expect("the structure is valid");
    let extensions = validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
        .expect("the extensions are accepted");
    match resolve_population_references(&structure, &extensions) {
        Ok(()) => Vec::new(),
        Err(errors) => errors
            .iter()
            .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
            .collect(),
    }
}

fn customers() -> Vec<(&'static str, String)> {
    vec![("E0C-22", CUSTOMERS_DATA.to_owned())]
}

#[test]
fn pop_4_3_the_fields_of_valid_populations_resolve() {
    assert!(missing(MODEL).is_empty());
}

#[test]
fn pop_4_3_pop_4_6_e0c_22_a_field_inside_required_when_is_missing() {
    assert_eq!(
        missing(conformance_model("e0-populations-missing-field-001")),
        customers()
    );
}

#[test]
fn pop_4_3_e0c_22_every_key_that_names_a_field_is_resolved() {
    let entity = VALIDITY.replace(r#"["customer_id"]"#, r#"["customer_id", "customer"]"#);
    assert_eq!(missing(&with_validity(&entity)), customers());
    let valid_to = VALIDITY.replace(r#""valid_to": "valid_to""#, r#""valid_to": "ends_on""#);
    assert_eq!(missing(&with_validity(&valid_to)), customers());
    let period = PERIODIC.replace("period_start", "period_end");
    let model = with_populations(&format!("[{VALIDITY}]"), &format!("[{period}]"));
    assert_eq!(missing(&model), [("E0C-22", ACCOUNTS_DATA.to_owned())]);
    let condition = VALIDITY.replace(r#""active""#, r#""active AND NOT archived""#);
    assert_eq!(missing(&with_validity(&condition)), customers());
}

#[test]
fn pop_4_3_field_names_are_declared_names_compared_after_normalization() {
    let regular = VALIDITY.replace(r#"["customer_id"]"#, r#"["Customer_ID"]"#);
    assert!(missing(&with_validity(&regular)).is_empty());
    let delimited_upper = VALIDITY.replace(r#"["customer_id"]"#, r#"["\"CUSTOMER_ID\""]"#);
    assert!(missing(&with_validity(&delimited_upper)).is_empty());
    let delimited_lower = VALIDITY.replace(r#"["customer_id"]"#, r#"["\"customer_id\""]"#);
    assert_eq!(missing(&with_validity(&delimited_lower)), customers());
    let condition = VALIDITY.replace(r#""active""#, r#""ACTIVE OR \"ACTIVE\"""#);
    assert!(missing(&with_validity(&condition)).is_empty());
}

#[test]
fn pop_4_4_conditions_left_to_e0f_are_not_e0c_22() {
    for condition in ["customers.archived", "active AND", "SUM(archived) > 0"] {
        let item = VALIDITY.replace(r#""active""#, &format!(r#""{condition}""#));
        let found = missing(&with_validity(&item));
        let expected = if condition.starts_with("SUM") {
            customers()
        } else {
            Vec::new()
        };
        assert_eq!(found, expected, "{condition}");
    }
}

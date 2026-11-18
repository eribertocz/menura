//! The `over` and the population of additivity declarations in E0.f: ADD-5.1.5, ADD-5.1.7,
//! `E0C-12` and `E0C-17`.

use semantic_compiler_contracts::admit_catalog_schema;
use semantic_compiler_contracts::types::audit_record::OssieFormat;

use crate::extensions::additivity::fixture::{
    CATALOG, DEBT_BALANCE_DATA, MODEL, conformance_model, with_additivity, with_example,
};
use crate::extensions::{DEFAULT_NAMESPACE, validate_extensions};
use crate::read::read_document;
use crate::structure::validate_structure;
use crate::typing::additivity::type_additivity;
use crate::typing::type_expressions;

/// The population of `customers` in every `e0-additivity*` model.
const VALIDITY: &str = r#""mode": "validity", "entity": ["customer_id"], "valid_from": "valid_from", "valid_to": "valid_to", "partial_period": "any_day""#;

/// The code and JSON Pointer of every diagnostic of the additivity declarations of `model`.
fn problems(model: &str) -> Vec<(&'static str, String)> {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure =
        validate_structure(&document, OssieFormat::Release, None).expect("the structure is valid");
    let catalog = admit_catalog_schema(CATALOG).expect("the catalog schema is admitted");
    let extensions = validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
        .expect("the extensions are accepted");
    let typing = type_expressions(&document, &structure, &catalog);
    match type_additivity(&structure, &typing, &extensions) {
        Ok(()) => Vec::new(),
        Err(errors) => errors
            .iter()
            .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
            .collect(),
    }
}

fn debt_balance(codes: &[&'static str]) -> Vec<(&'static str, String)> {
    codes
        .iter()
        .map(|code| (*code, DEBT_BALANCE_DATA.to_owned()))
        .collect()
}

fn example(replacements: &[(&str, &str)]) -> Vec<(&'static str, String)> {
    problems(&with_example(replacements))
}

/// A declaration over `over` with only the required keys and `extra`.
fn declaration(over: &str, extra: &str) -> Vec<(&'static str, String)> {
    problems(&with_additivity(&format!(
        r#"{{"type": "semi_additive", "over": "{over}", "entity": ["debt.customer_id"], "aggregation": "min", "nulls": "keep"{extra}}}"#
    )))
}

#[test]
fn add_5_1_5_add_5_1_7_the_example_has_no_diagnostic() {
    assert!(problems(MODEL).is_empty());
    for model in ["e0-additivity-unreachable-001", "e0-additivity-conflicts"] {
        assert!(problems(conformance_model(model)).is_empty(), "{model}");
    }
}

#[test]
fn add_5_1_5_e0c_12_a_time_option_with_another_over() {
    assert_eq!(
        problems(conformance_model("e0-additivity-time-kind-001")),
        debt_balance(&["E0C-12"])
    );
    assert!(declaration("customers.category", "").is_empty());
    for extra in [
        r#", "population": "active_customers""#,
        r#", "missing": "zero""#,
        r#", "unexpected": "include""#,
    ] {
        assert_eq!(
            declaration("customers.category", extra),
            debt_balance(&["E0C-12"]),
            "{extra}"
        );
    }
    let ordered = problems(&with_additivity(
        r#"{"type": "semi_additive", "over": "debt.balance", "entity": ["debt.customer_id"], "aggregation": "first", "selection": "per_entity", "nulls": "as_missing"}"#,
    ));
    assert_eq!(ordered, debt_balance(&["E0C-12"]));
}

#[test]
fn add_5_1_5_e0c_12_a_time_over_requires_the_time_options() {
    assert_eq!(declaration("debt.period", ""), debt_balance(&["E0C-12"]));
    for key in [
        r#""snapshot_grain": "month", "#,
        r#""population": "active_customers", "#,
        r#""unexpected": "exclude", "#,
    ] {
        assert_eq!(example(&[(key, "")]), debt_balance(&["E0C-12"]), "{key}");
    }
    assert_eq!(
        example(&[(
            r#""missing": "carry_forward", "max_carry_forward": 2, "on_stale": "error", "#,
            ""
        )]),
        debt_balance(&["E0C-12"])
    );
    assert!(example(&[(r#""debt.period""#, r#""customers.valid_from""#)]).is_empty());
}

#[test]
fn add_5_1_5_e0c_12_is_checked_only_on_a_typed_over() {
    let untyped = with_example(&[(r#""unexpected": "exclude", "#, "")]).replacen(
        "expression: period}",
        "expression: no_such_column}",
        1,
    );
    assert!(problems(&untyped).is_empty());
}

#[test]
fn add_5_1_7_e0c_17_a_population_that_does_not_exist() {
    assert_eq!(
        problems(conformance_model("e0-additivity-population-001")),
        debt_balance(&["E0C-17"])
    );
    assert!(example(&[(r#""active_customers""#, r#""ACTIVE_CUSTOMERS""#)]).is_empty());
    assert_eq!(
        example(&[(r#""active_customers""#, r#""\"active_customers\"""#)]),
        debt_balance(&["E0C-17"])
    );
}

#[test]
fn add_5_1_7_e0c_17_the_entity_matches_in_number_order_and_kind() {
    assert_eq!(
        example(&[(
            r#"["debt.customer_id"]"#,
            r#"["debt.customer_id", "debt.balance"]"#
        )]),
        debt_balance(&["E0C-17"])
    );
    assert_eq!(
        example(&[(r#"["debt.customer_id"]"#, r#"["customers.category"]"#)]),
        debt_balance(&["E0C-17"])
    );
    assert!(example(&[(r#"["debt.customer_id"]"#, r#"["debt.balance"]"#)]).is_empty());
}

#[test]
fn add_5_1_7_e0c_17_a_periodic_population_has_the_snapshot_grain() {
    let periodic = |grain: &str| {
        let population = format!(
            r#""mode": "periodic", "entity": ["customer_id"], "period": "valid_from", "grain": "{grain}""#
        );
        assert!(MODEL.contains(VALIDITY), "the model has the population");
        MODEL.replacen(VALIDITY, &population, 1)
    };
    assert!(problems(&periodic("month")).is_empty());
    assert_eq!(problems(&periodic("quarter")), debt_balance(&["E0C-17"]));
}

#[test]
fn add_5_1_5_add_5_1_7_both_codes_in_the_order_of_the_codes() {
    assert_eq!(
        example(&[
            (r#""unexpected": "exclude", "#, ""),
            (r#""active_customers""#, r#""former_customers""#),
        ]),
        debt_balance(&["E0C-12", "E0C-17"])
    );
}

//! The fields of additivity declarations in E0.e: ADD-5.1.6 and `E0C-16`.

use semantic_compiler_contracts::admit_catalog_schema;
use semantic_compiler_contracts::types::audit_record::OssieFormat;

use crate::extensions::additivity::fixture::{
    CATALOG, DEBT_BALANCE_DATA, MODEL, conformance_model, with_debt_balance_expression,
    with_example,
};
use crate::extensions::summaries::SummaryDatasets;
use crate::extensions::{DEFAULT_NAMESPACE, validate_extensions};
use crate::read::read_document;
use crate::references::resolve_references;
use crate::relationships::additivity::check_additivity_reachability;
use crate::relationships::analyze_relationships;
use crate::structure::validate_structure;

/// The code and JSON Pointer of every `E0C-16` diagnostic of the release-format `model`.
fn problems(model: &str) -> Vec<(&'static str, String)> {
    problems_in(model, OssieFormat::Release)
}

/// The code and JSON Pointer of every `E0C-16` diagnostic of `model`, read in `format`.
fn problems_in(model: &str, format: OssieFormat) -> Vec<(&'static str, String)> {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure = validate_structure(&document, format, None).expect("the structure is valid");
    let extensions = validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
        .expect("the extensions are accepted");
    let catalog = admit_catalog_schema(CATALOG).expect("the catalog schema is admitted");
    let references =
        resolve_references(&document, &structure, &catalog, &SummaryDatasets::default())
            .expect("the references resolve");
    let relationships = analyze_relationships(
        &document,
        &structure.semantic_model,
        &extensions,
        &references,
    );
    match check_additivity_reachability(&structure, &extensions, &references, &relationships) {
        Ok(()) => Vec::new(),
        Err(errors) => errors
            .iter()
            .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
            .collect(),
    }
}

fn debt_balance() -> Vec<(&'static str, String)> {
    vec![("E0C-16", DEBT_BALANCE_DATA.to_owned())]
}

/// What the example of ADD-8 changed by `replacements` finds.
fn example(replacements: &[(&str, &str)]) -> Vec<(&'static str, String)> {
    problems(&with_example(replacements))
}

#[test]
fn add_5_1_6_the_example_is_reachable() {
    assert!(problems(MODEL).is_empty());
    for model in [
        "e0-additivity-time-kind-001",
        "e0-additivity-population-001",
        "e0-additivity-conflicts",
    ] {
        assert!(problems(conformance_model(model)).is_empty(), "{model}");
    }
}

#[test]
fn add_5_1_6_e0c_16_a_field_not_reachable_from_the_home_dataset() {
    assert_eq!(
        problems(conformance_model("e0-additivity-unreachable-001")),
        [(
            "E0C-16",
            "/semantic_model/0/metrics/1/custom_extensions/0/data".to_owned()
        )]
    );
}

#[test]
fn add_5_1_6_a_field_of_a_reached_dataset_is_reachable() {
    assert!(
        example(&[(
            r#"["debt.customer_id"]"#,
            r#"["debt.customer_id", "customers.category"]"#
        )])
        .is_empty()
    );
    assert!(example(&[(r#""debt.period""#, r#""\"CUSTOMERS\".valid_from""#)]).is_empty());
}

#[test]
fn add_5_1_6_e0c_16_a_field_or_dataset_that_does_not_exist() {
    assert_eq!(
        example(&[(r#""debt.period""#, r#""debt.month""#)]),
        debt_balance()
    );
    assert_eq!(
        example(&[(r#"["debt.customer_id"]"#, r#"["loans.customer_id"]"#)]),
        debt_balance()
    );
    assert_eq!(
        example(&[(r#""debt.period""#, r#""\"debt\".period""#)]),
        debt_balance()
    );
}

#[test]
fn add_5_1_6_one_diagnostic_per_declaration() {
    let model = with_example(&[
        (r#""debt.period""#, r#""debt.month""#),
        (r#"["debt.customer_id"]"#, r#"["loans.customer_id"]"#),
    ]);
    assert_eq!(problems(&model), debt_balance());
}

#[test]
fn add_5_1_6_without_a_single_home_dataset_e0c_16_is_not_checked() {
    let unreachable = with_example(&[(r#""debt.period""#, r#""debt.month""#)]);
    for expression in [
        "SUM(debt.balance) / COUNT(customers.customer_id)",
        "COUNT(*)",
        "1 + 2",
        "SUM(debt.balance",
    ] {
        let model = with_debt_balance_expression(expression);
        let model = model.replacen(r#""over": "debt.period""#, r#""over": "debt.month""#, 1);
        assert_ne!(model, with_debt_balance_expression(expression));
        assert!(problems(&model).is_empty(), "{expression}");
    }
    assert_eq!(problems(&unreachable), debt_balance());
}

#[test]
fn add_5_1_6_a_metric_unusable_before_e0e_is_not_checked() {
    let model = with_debt_balance_expression("SUM(debt.balance) + SUM(debt.missing_field)");
    let model = model.replacen(r#""over": "debt.period""#, r#""over": "debt.month""#, 1);
    assert!(problems(&model).is_empty());
}

/// The development-line model of `e0-additivity-development-unreachable-001`.
const DEVELOPMENT_UNREACHABLE: &str = include_str!(
    "../../../../../conformance/models/e0-additivity-development-unreachable-001/model.yaml"
);

#[test]
fn add_5_1_6_e0g_1_4_e0c_16_the_home_dataset_of_a_dataset_metric_is_its_own_dataset() {
    assert_eq!(
        problems_in(DEVELOPMENT_UNREACHABLE, OssieFormat::Development),
        [(
            "E0C-16",
            "/datasets/1/metrics/0/nonportable_extensions/0/data".to_owned()
        )]
    );
    let reachable = DEVELOPMENT_UNREACHABLE
        .replace("over: debt.period", "over: customers.valid_from")
        .replace(
            "entity: [debt.customer_id]",
            "entity: [customers.customer_id]",
        );
    assert!(problems_in(&reachable, OssieFormat::Development).is_empty());
}

#[test]
fn add_5_1_6_a_dataset_metric_unusable_before_e0e_is_not_checked() {
    for expression in [
        "{sql: SUM(missing_field)}",
        "{sql: COUNT(*)}\n        datatype: Opaque",
    ] {
        let model = DEVELOPMENT_UNREACHABLE.replacen("{sql: COUNT(*)}", expression, 1);
        assert_ne!(model, DEVELOPMENT_UNREACHABLE);
        assert!(
            problems_in(&model, OssieFormat::Development).is_empty(),
            "{expression}"
        );
    }
}

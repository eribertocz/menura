//! The E0 stage function: E0-2.1, E0-3.1 to E0-3.3, E0-4.1 to E0-4.4, STG-3.4, STG-3.6 and
//! CAT-9.1.
//!
//! The model document and catalog schema of the `sample` conformance model are read at compile
//! time; each test edits the model text to introduce the problems it needs.

use menura_contracts::types::audit_record::{LoadOptions, OssieFormat};
use menura_contracts::types::compiled_plan::{Diagnostic, InputDocument};
use menura_model::internal_model::InternalModel;

use super::{LoadRejection, load_model};

const MODEL: &str = include_str!("../../../../conformance/models/sample/model.yaml");
const CATALOG: &str = include_str!("../../../../conformance/models/sample/catalog-schema.json");

/// A field of `orders` whose column the catalog schema does not describe (`E0D-03`, `W0D-01`).
const MISSING_COLUMN_FIELD: (&str, &str) = (
    "          - name: amount\n",
    "          - name: discount\n            expression: {dialects: [{dialect: ANSI_SQL, expression: discount}]}\n          - name: amount\n",
);
/// The `status` field with only a `SNOWFLAKE` variant (`W0B-01`, `W0B-02`).
const NO_ANSI_VARIANT: (&str, &str) = (
    "expression: {dialects: [{dialect: ANSI_SQL, expression: status}]}",
    "expression: {dialects: [{dialect: SNOWFLAKE, expression: status}]}",
);
/// A metric that sums `NULL`, which has no type (`E0F-03`, `W0F-01`).
const MISTYPED_METRIC: (&str, &str) = (
    "      - name: total_balance\n",
    "      - name: status_sum\n        expression: {dialects: [{dialect: ANSI_SQL, expression: SUM(NULL)}]}\n      - name: total_balance\n",
);
/// A metric with a field reference outside every aggregate (E0G-2.1, `W0G-01`).
const UNAGGREGATED_METRIC: (&str, &str) = (
    "      - name: total_amount\n",
    "      - name: raw_amount\n        expression: {dialects: [{dialect: ANSI_SQL, expression: orders.order_id}]}\n      - name: total_amount\n",
);
/// The `amount` field with an expression that does not parse (`E0F-01`, `W0F-01`).
const UNPARSABLE_FIELD: (&str, &str) = (
    "expression: {dialects: [{dialect: ANSI_SQL, expression: amount}]}",
    "expression: {dialects: [{dialect: ANSI_SQL, expression: 'amount +'}]}",
);
/// An unknown key in the extension entry of `orders` (`E0C-02`, rejects in E0.c).
const UNKNOWN_EXTENSION_KEY: (&str, &str) = (
    r#""access": {"mode": "roles", "roles": ["analyst"]}}'
      - name: balances"#,
    r#""access": {"mode": "roles", "roles": ["analyst"]}, "colour": 1}'
      - name: balances"#,
);
/// The source of `balances` is not in the catalog schema (`E0D-02`, rejects in E0.d).
const SOURCE_NOT_IN_CATALOG: (&str, &str) = ("source: shop.balances", "source: shop.missing");
/// The additivity `over` names a field `balances` does not have (`E0C-16`, an extension check
/// of E0.e).
const UNREACHABLE_OVER: (&str, &str) = (
    r#""over": "balances.branch""#,
    r#""over": "balances.nowhere""#,
);

/// `MODEL` with every edit applied; each `from` must occur in it.
fn edited(edits: &[(&str, &str)]) -> String {
    let mut model = MODEL.to_owned();
    for (from, to) in edits {
        assert!(model.contains(from), "the model contains `{from}`");
        model = model.replacen(from, to, 1);
    }
    model
}

fn options() -> LoadOptions {
    LoadOptions {
        grant_authenticated_to_all: false,
        ossie_format: OssieFormat::Release,
        namespace: "menura".to_owned(),
        model_name: None,
        data_source: None,
    }
}

fn load(model: &str) -> Result<InternalModel, LoadRejection> {
    load_model(model.as_bytes(), CATALOG.as_bytes(), &options()).map(|loaded| loaded.model)
}

fn loaded(model: &str) -> InternalModel {
    match load(model) {
        Ok(model) => model,
        Err(rejection) => panic!("the model loads, but: {rejection}"),
    }
}

fn rejected(model: &str, catalog: &[u8]) -> LoadRejection {
    match load_model(model.as_bytes(), catalog, &options()) {
        Ok(_) => panic!("the model is rejected"),
        Err(rejection) => rejection,
    }
}

fn codes(diagnostics: &[Diagnostic]) -> Vec<&str> {
    diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

fn warning_codes(model: &InternalModel) -> Vec<&str> {
    model
        .parts()
        .warnings
        .iter()
        .map(|warning| warning.code.as_str())
        .collect()
}

#[test]
fn e0_2_1_a_valid_model_loads_without_security_context_or_catalog_state() {
    let model = loaded(MODEL);
    assert!(model.parts().unusable.is_empty());
    assert!(model.parts().warnings.is_empty());
}

#[test]
fn e0_2_1_the_admitted_catalog_schema_is_returned_with_the_model() {
    let loaded = load_model(MODEL.as_bytes(), CATALOG.as_bytes(), &options()).unwrap();
    assert_eq!(
        loaded.catalog_schema.document_id.origin,
        "conformance:sample"
    );
}

#[test]
fn stg_3_4_independent_substages_report_every_problem_in_one_pass() {
    let model = loaded(&edited(&[
        NO_ANSI_VARIANT,
        MISSING_COLUMN_FIELD,
        MISTYPED_METRIC,
        UNAGGREGATED_METRIC,
    ]));
    assert_eq!(
        warning_codes(&model),
        ["W0B-02", "W0B-01", "W0D-01", "W0F-01", "W0G-01"]
    );
}

#[test]
fn e0_4_3_diagnostics_are_in_substage_order_before_document_order() {
    let rejection = rejected(
        &edited(&[SOURCE_NOT_IN_CATALOG, UNKNOWN_EXTENSION_KEY]),
        CATALOG.as_bytes(),
    );
    // `E0C-02` is at `orders`, `E0D-02` at `balances`; E0.c comes before E0.d whatever the
    // document order.
    assert_eq!(codes(&rejection.errors), ["E0C-02", "E0D-02"]);
}

#[test]
fn e0_4_3_diagnostics_of_one_substage_are_in_document_order() {
    let model = loaded(&edited(&[
        MISSING_COLUMN_FIELD,
        UNPARSABLE_FIELD,
        MISTYPED_METRIC,
    ]));
    let pointers: Vec<&str> = model
        .parts()
        .warnings
        .iter()
        .map(|warning| warning.location.pointer.as_str())
        .collect();
    assert_eq!(
        pointers,
        [
            "/semantic_model/0/datasets/0/fields/2/expression",
            "/semantic_model/0/datasets/0/fields/3/expression",
            "/semantic_model/0/metrics/1/expression",
        ]
    );
    assert_eq!(warning_codes(&model), ["W0D-01", "W0F-01", "W0F-01"]);
}

#[test]
fn e0_3_1_an_unusable_diagnostic_does_not_make_its_substage_fail() {
    let model = loaded(&edited(&[MISSING_COLUMN_FIELD, UNAGGREGATED_METRIC]));
    assert_eq!(warning_codes(&model), ["W0D-01", "W0G-01"]);
}

#[test]
fn e0_4_4_a_missing_reference_never_rejects_the_model() {
    let model = loaded(&edited(&[MISSING_COLUMN_FIELD]));
    assert_eq!(model.parts().unusable.len(), 1);
}

#[test]
fn e0_4_1_an_expression_that_does_not_parse_reports_w0f_01() {
    let model = loaded(&edited(&[UNPARSABLE_FIELD]));
    let warning = &model.parts().warnings[0];
    assert_eq!(warning.code.as_str(), "W0F-01");
    assert_eq!(
        warning.location.pointer,
        "/semantic_model/0/datasets/0/fields/2/expression"
    );
}

#[test]
fn e0_4_2a_a_propagated_element_reports_no_new_warning() {
    // `total_amount` reads the unparsable `orders.amount`.
    let model = loaded(&edited(&[UNPARSABLE_FIELD]));
    assert_eq!(warning_codes(&model), ["W0F-01"]);
    let metric = model
        .parts()
        .unusable
        .iter()
        .find(|element| element.location.pointer == "/semantic_model/0/metrics/0")
        .expect("total_amount is unusable");
    assert_eq!(metric.load_warning, None);
}

#[test]
fn e0_4_1_every_unusable_element_reports_its_load_warning() {
    let model = loaded(&edited(&[
        NO_ANSI_VARIANT,
        MISSING_COLUMN_FIELD,
        UNPARSABLE_FIELD,
        MISTYPED_METRIC,
        UNAGGREGATED_METRIC,
    ]));
    let parts = model.parts();
    let mut with_warning = 0;
    for element in &parts.unusable {
        let Some(code) = &element.load_warning else {
            continue;
        };
        with_warning += 1;
        assert!(
            parts.warnings.iter().any(|warning| warning.code == *code
                && warning
                    .location
                    .pointer
                    .starts_with(&element.location.pointer)),
            "`{}` reports `{}`",
            element.location.pointer,
            code.as_str()
        );
    }
    assert_eq!(with_warning, 5);
}

#[test]
fn e0_3_2_an_invalid_catalog_schema_comes_first_and_e0a_to_e0c_still_run() {
    let rejection = rejected(&edited(&[UNKNOWN_EXTENSION_KEY]), b"{}");
    assert_eq!(codes(&rejection.errors), ["ECAT-02", "E0C-02"]);
    let location = rejection.errors[0].location.as_ref().unwrap();
    assert_eq!(location.document, InputDocument::CatalogSchema);
}

#[test]
fn e0_3_2_without_an_admitted_catalog_schema_e0d_does_not_run() {
    let rejection = rejected(&edited(&[NO_ANSI_VARIANT, MISSING_COLUMN_FIELD]), b"{}");
    assert_eq!(codes(&rejection.errors), ["ECAT-02"]);
    // `W0B-02` is at the expression, `W0B-01` at its variant inside it (E0-4.3).
    assert_eq!(codes(&rejection.warnings), ["W0B-02", "W0B-01"]);
}

#[test]
fn cat_9_1_a_catalog_schema_that_is_not_json_is_ecat_01_at_the_empty_pointer() {
    for catalog in [&b"not json"[..], &[0xff, 0xfe][..]] {
        let rejection = rejected(MODEL, catalog);
        assert_eq!(codes(&rejection.errors), ["ECAT-01"]);
        let location = rejection.errors[0].location.as_ref().unwrap();
        assert_eq!(location.document, InputDocument::CatalogSchema);
        assert_eq!(location.pointer, "");
    }
}

#[test]
fn e0_3_3_when_e0c_rejects_the_own_checks_of_e0d_to_e0g_still_run() {
    let rejection = rejected(
        &edited(&[
            UNKNOWN_EXTENSION_KEY,
            MISSING_COLUMN_FIELD,
            MISTYPED_METRIC,
            UNAGGREGATED_METRIC,
        ]),
        CATALOG.as_bytes(),
    );
    assert_eq!(codes(&rejection.errors), ["E0C-02"]);
    assert_eq!(codes(&rejection.warnings), ["W0D-01", "W0F-01", "W0G-01"]);
}

#[test]
fn e0_3_3_an_extension_check_of_e0e_runs_when_e0c_succeeds() {
    let rejection = rejected(&edited(&[UNREACHABLE_OVER]), CATALOG.as_bytes());
    assert_eq!(codes(&rejection.errors), ["E0C-16"]);
}

#[test]
fn e0_3_3_when_e0c_rejects_the_extension_checks_of_e0d_to_e0g_do_not_run() {
    let rejection = rejected(
        &edited(&[UNKNOWN_EXTENSION_KEY, UNREACHABLE_OVER]),
        CATALOG.as_bytes(),
    );
    assert_eq!(codes(&rejection.errors), ["E0C-02"]);
}

#[test]
fn e0_3_when_e0d_rejects_the_substages_that_depend_on_it_do_not_run() {
    let rejection = rejected(
        &edited(&[SOURCE_NOT_IN_CATALOG, UNAGGREGATED_METRIC]),
        CATALOG.as_bytes(),
    );
    assert_eq!(codes(&rejection.errors), ["E0D-02"]);
    assert!(codes(&rejection.warnings).is_empty());
}

#[test]
fn e0_3_a_model_that_cannot_be_read_stops_at_e0a() {
    let rejection = rejected("a: [", CATALOG.as_bytes());
    assert_eq!(codes(&rejection.errors), ["E0A-02"]);
    let location = rejection.errors[0].location.as_ref().unwrap();
    assert_eq!(location.document, InputDocument::Model);
    assert!(location.line.is_some());
}

#[test]
fn stg_3_6_every_diagnostic_has_a_code_a_message_and_a_location() {
    let rejection = rejected(
        &edited(&[
            UNKNOWN_EXTENSION_KEY,
            NO_ANSI_VARIANT,
            MISSING_COLUMN_FIELD,
            UNPARSABLE_FIELD,
        ]),
        CATALOG.as_bytes(),
    );
    for diagnostic in rejection.errors.iter().chain(&rejection.warnings) {
        assert!(!diagnostic.message.is_empty());
        let location = diagnostic.location.as_ref().unwrap();
        assert_eq!(location.document, InputDocument::Model);
        assert!(location.pointer.starts_with("/semantic_model/0"));
        assert!(location.line.is_some() && location.column.is_some());
    }
}

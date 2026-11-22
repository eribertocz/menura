//! `validate`, `load` and `with_state`: PLN-5.3, ARC-5.1, ARC-5.2, CAT-8.6, CAT-8.8, CAT-9.1
//! and SUM-6.1.8.

use menura_core::{
    CompileErrorContract, CompileErrorDocument, Handle, InputDocument, LoadOptions, load, validate,
    with_state,
};

const SAMPLE_MODEL: &str = include_str!("../../../conformance/models/sample/model.yaml");
const SAMPLE_CATALOG: &str = include_str!("../../../conformance/models/sample/catalog-schema.json");
const SUMMARIES_MODEL: &str = include_str!("../../../conformance/models/e0-summaries/model.yaml");
const SUMMARIES_CATALOG: &str =
    include_str!("../../../conformance/models/e0-summaries/catalog-schema.json");
/// A month summary of `orders`, whose catalog state builds it from day periods.
const MONTHLY_MODEL: &str =
    include_str!("../../../conformance/models/cat-state-summary-built-from-days-001/model.yaml");
const MONTHLY_CATALOG: &str = include_str!(
    "../../../conformance/models/cat-state-summary-built-from-days-001/catalog-schema.json"
);
const MONTHLY_BUILT_FROM_DAYS: &str = include_str!(
    "../../../conformance/models/cat-state-summary-built-from-days-001/catalog-state.json"
);
const MONTHLY_BUILT_FROM_WEEKS: &str = include_str!(
    "../../../conformance/models/cat-state-summary-built-from-weeks-001/catalog-state.json"
);
const MONTHLY_BUILD_AND_CHECK_GRAINS: &str = include_str!(
    "../../../conformance/models/cat-state-summary-build-grains-001/catalog-state.json"
);

/// A field of `orders` whose column the catalog schema does not describe (`W0D-01`).
fn sample_with_missing_column() -> String {
    SAMPLE_MODEL.replacen(
        "          - name: amount\n",
        "          - name: discount\n            expression: {dialects: [{dialect: ANSI_SQL, expression: discount}]}\n          - name: amount\n",
        1,
    )
}

/// A catalog state for the `e0-summaries` schema: `source` has an `aggregate_build` whose
/// periods are `periods` (the first two are the coverage).
fn summaries_state(source: &str, periods: &[&str]) -> String {
    let versions: Vec<String> = periods
        .iter()
        .map(|period| {
            format!(
                r#"{{"period": "{period}", "version": {{"origin": "shop", "snapshot": "{period}"}}}}"#
            )
        })
        .collect();
    let versions = versions.join(", ");
    let hash = "0".repeat(64);
    format!(
        r#"{{
  "contract": "menura.catalog_state",
  "contract_version": "1.0.0",
  "document_id": {{"origin": "conformance:e0-summaries", "snapshot": "state-1"}},
  "schema_document_id": {{"origin": "conformance:e0-summaries", "snapshot": "e0-summaries"}},
  "relations": [
    {{
      "source": "{source}",
      "row_count_estimate": 10,
      "aggregate_build": {{
        "definition_hash": "{hash}",
        "coverage": {{"from": "{from}", "to": "{to}"}},
        "built_from": [{versions}],
        "integrity_check": {{"status": "passed", "definition_hash": "{hash}", "verified": [{versions}]}},
        "time_zone": "UTC"
      }}
    }}
  ]
}}"#,
        from = periods[0],
        to = periods[periods.len() - 1],
    )
}

/// The handle of the `e0-summaries` model, which loads without warnings.
fn summaries_handle() -> Result<Handle, CompileErrorDocument> {
    load(
        SUMMARIES_MODEL.as_bytes(),
        SUMMARIES_CATALOG.as_bytes(),
        None,
        &LoadOptions::default(),
    )
}

fn warning_codes(handle: &Handle) -> Vec<String> {
    handle
        .warnings()
        .map(|warning| warning.code.clone())
        .collect()
}

#[test]
fn pln_5_3_validate_returns_a_validation_report_with_empty_errors() {
    let report = validate(
        sample_with_missing_column().as_bytes(),
        SAMPLE_CATALOG.as_bytes(),
        &LoadOptions::default(),
    );
    assert_eq!(report.contract, CompileErrorContract::ValidationReport);
    assert_eq!(report.contract_version.to_string(), "1.1.0");
    assert!(report.errors.is_empty());
    let codes: Vec<&str> = report.warnings.iter().map(|w| w.code.as_str()).collect();
    assert_eq!(codes, ["W0D-01"]);
    let location = report.warnings[0].location.as_ref().unwrap();
    assert_eq!(location.document, InputDocument::Model);
}

#[test]
fn pln_5_3_validate_reports_the_errors_of_a_model_that_does_not_load() {
    let report = validate(SAMPLE_MODEL.as_bytes(), b"[]", &LoadOptions::default());
    assert_eq!(report.contract, CompileErrorContract::ValidationReport);
    assert_eq!(report.errors[0].code, "ECAT-02");
}

#[test]
fn arc_5_1_load_returns_a_compile_error_when_the_model_does_not_load() {
    let Err(error) = load(
        SAMPLE_MODEL.as_bytes(),
        b"[]",
        None,
        &LoadOptions::default(),
    ) else {
        panic!("the model does not load");
    };
    assert_eq!(error.contract, CompileErrorContract::CompileError);
    assert_eq!(error.errors[0].code, "ECAT-02");
}

#[test]
fn arc_5_1_load_returns_a_handle_with_the_load_warnings() {
    let handle = load(
        sample_with_missing_column().as_bytes(),
        SAMPLE_CATALOG.as_bytes(),
        None,
        &LoadOptions::default(),
    )
    .unwrap();
    assert_eq!(warning_codes(&handle), ["W0D-01"]);
    assert!(handle.catalog_state().is_none());
    assert_eq!(handle.catalog_schema().document_id.snapshot, "sample-001");
    assert_eq!(handle.model().parts().warnings.len(), 1);
}

#[test]
fn sum_6_1_8_a_summary_build_of_the_coverage_grain_is_admitted() {
    let state = summaries_state("shop.main.orders_daily", &["2026-10-01", "2026-10-02"]);
    let handle = with_state(&summaries_handle().unwrap(), state.as_bytes());
    assert!(handle.catalog_state().is_some());
    assert!(warning_codes(&handle).is_empty());
}

#[test]
fn cat_8_8_a_summary_build_on_a_relation_that_is_not_a_summary_is_ignored_with_wcat_03() {
    let state = summaries_state("shop.main.orders", &["2026-10-01", "2026-10-02"]);
    let handle = with_state(&summaries_handle().unwrap(), state.as_bytes());
    assert!(handle.catalog_state().is_none());
    assert_eq!(warning_codes(&handle), ["WCAT-03"]);
    let location = handle.warnings().next().unwrap().location.clone().unwrap();
    assert_eq!(location.document, InputDocument::CatalogState);
    assert_eq!(location.pointer, "/relations/0/aggregate_build");
}

#[test]
fn cat_8_8_a_summary_build_with_periods_of_another_grain_is_ignored_with_wcat_03() {
    let state = summaries_state("shop.main.orders_daily", &["2026-09", "2026-10"]);
    let handle = with_state(&summaries_handle().unwrap(), state.as_bytes());
    assert!(handle.catalog_state().is_none());
    assert_eq!(warning_codes(&handle), ["WCAT-03"]);
}

#[test]
fn cat_8_8_load_checks_the_summary_builds_of_its_catalog_state() {
    let state = summaries_state("shop.main.orders", &["2026-10-01"]);
    let handle = load(
        SUMMARIES_MODEL.as_bytes(),
        SUMMARIES_CATALOG.as_bytes(),
        Some(state.as_bytes()),
        &LoadOptions::default(),
    )
    .unwrap();
    assert!(handle.catalog_state().is_none());
    assert_eq!(warning_codes(&handle), ["WCAT-03"]);
}

/// The handle of the month summary model with the catalog state `state`.
fn monthly_handle(state: &str) -> Result<Handle, CompileErrorDocument> {
    load(
        MONTHLY_MODEL.as_bytes(),
        MONTHLY_CATALOG.as_bytes(),
        Some(state.as_bytes()),
        &LoadOptions::default(),
    )
}

#[test]
fn cat_8_8_a_month_summary_built_from_day_periods_is_admitted() {
    let handle = monthly_handle(MONTHLY_BUILT_FROM_DAYS).unwrap();
    assert!(handle.catalog_state().is_some());
    assert!(warning_codes(&handle).is_empty());
}

#[test]
fn cat_8_8_a_month_summary_built_from_week_periods_is_ignored_with_wcat_03() {
    let handle = monthly_handle(MONTHLY_BUILT_FROM_WEEKS).unwrap();
    assert!(handle.catalog_state().is_none());
    assert_eq!(warning_codes(&handle), ["WCAT-03"]);
    let location = handle.warnings().next().unwrap().location.clone().unwrap();
    assert_eq!(location.document, InputDocument::CatalogState);
    assert_eq!(
        location.pointer,
        "/relations/1/aggregate_build/built_from/0/period"
    );
}

#[test]
fn cat_8_6_a_build_and_a_check_of_different_grains_are_ignored_with_wcat_03() {
    let handle = monthly_handle(MONTHLY_BUILD_AND_CHECK_GRAINS).unwrap();
    assert!(handle.catalog_state().is_none());
    assert_eq!(warning_codes(&handle), ["WCAT-03"]);
    let location = handle.warnings().next().unwrap().location.clone().unwrap();
    assert_eq!(
        location.pointer,
        "/relations/1/aggregate_build/integrity_check/verified/0/period"
    );
}

#[test]
fn cat_8_8_a_month_summary_with_day_coverage_is_ignored_with_wcat_03() {
    let state = MONTHLY_BUILT_FROM_DAYS.replace(
        r#""from": "2026-11",
          "to": "2026-11""#,
        r#""from": "2026-11-01",
          "to": "2026-11-02""#,
    );
    assert_ne!(state, MONTHLY_BUILT_FROM_DAYS);
    let handle = monthly_handle(&state).unwrap();
    assert_eq!(warning_codes(&handle), ["WCAT-03"]);
    let location = handle.warnings().next().unwrap().location.clone().unwrap();
    assert_eq!(
        location.pointer,
        "/relations/1/aggregate_build/coverage/from"
    );
}

#[test]
fn arc_5_2_an_invalid_catalog_state_is_ignored_with_its_warning() {
    let handle = with_state(&summaries_handle().unwrap(), b"not json");
    assert!(handle.catalog_state().is_none());
    assert_eq!(warning_codes(&handle), ["WCAT-01"]);
}

#[test]
fn cat_9_1_a_catalog_state_that_is_not_utf8_is_wcat_01_at_the_empty_pointer() {
    let handle = with_state(&summaries_handle().unwrap(), &[0xff, 0xfe]);
    assert_eq!(warning_codes(&handle), ["WCAT-01"]);
    let location = handle.warnings().next().unwrap().location.clone().unwrap();
    assert_eq!(location.pointer, "");
}

#[test]
fn arc_5_2_a_state_of_another_schema_document_is_wcat_04() {
    let state = summaries_state("shop.main.orders_daily", &["2026-10-01"])
        .replace(r#""snapshot": "e0-summaries""#, r#""snapshot": "other""#);
    let handle = with_state(&summaries_handle().unwrap(), state.as_bytes());
    assert_eq!(warning_codes(&handle), ["WCAT-04"]);
}

#[test]
fn arc_5_1_with_state_returns_a_new_handle_and_keeps_the_model() {
    let handle = summaries_handle().unwrap();
    let state = summaries_state("shop.main.orders_daily", &["2026-10-01"]);
    let with = with_state(&handle, state.as_bytes());
    assert!(handle.catalog_state().is_none());
    assert!(with.catalog_state().is_some());
    assert_eq!(handle.model().hash(), with.model().hash());
    let replaced = with_state(&with, b"not json");
    assert!(replaced.catalog_state().is_none());
    assert_eq!(warning_codes(&replaced), ["WCAT-01"]);
}

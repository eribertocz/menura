//! The former contract name of the catalog documents (CON-0.5, WCAT-05): the document is
//! admitted, with its warning at `/contract`; a schema's warning comes before the E0 warnings
//! (E0-3.2), a state's after them (ARC-5.2).

use menura_core::{InputDocument, LoadOptions, load, validate, with_state};

const SAMPLE_MODEL: &str = include_str!("../../../conformance/models/sample/model.yaml");
const SAMPLE_CATALOG: &str = include_str!("../../../conformance/models/sample/catalog-schema.json");
const STATE_MODEL: &str =
    include_str!("../../../conformance/models/cat-state-valid-001/model.yaml");
const STATE_CATALOG: &str =
    include_str!("../../../conformance/models/cat-state-valid-001/catalog-schema.json");
const STATE: &str =
    include_str!("../../../conformance/models/cat-state-valid-001/catalog-state.json");

fn with_former_name(document: &str, local_name: &str) -> String {
    let name = format!("\"menura.{local_name}\"");
    assert!(document.contains(&name), "the document names {name}");
    document.replacen(&name, &format!("\"semantic_compiler.{local_name}\""), 1)
}

/// A field of `orders` whose column the catalog schema does not describe (`W0D-01`).
fn sample_with_missing_column() -> String {
    SAMPLE_MODEL.replacen(
        "          - name: amount\n",
        "          - name: discount\n            expression: {dialects: [{dialect: ANSI_SQL, expression: discount}]}\n          - name: amount\n",
        1,
    )
}

#[test]
fn con_0_5_wcat_05_a_former_name_schema_warns_before_the_e0_warnings() {
    let catalog = with_former_name(SAMPLE_CATALOG, "catalog_schema");
    let report = validate(
        sample_with_missing_column().as_bytes(),
        catalog.as_bytes(),
        &LoadOptions::default(),
    );
    assert!(report.errors.is_empty());
    let codes: Vec<&str> = report.warnings.iter().map(|w| w.code.as_str()).collect();
    assert_eq!(codes, ["WCAT-05", "W0D-01"]);
    let location = report.warnings[0].location.as_ref().unwrap();
    assert_eq!(location.document, InputDocument::CatalogSchema);
    assert_eq!(location.pointer, "/contract");
    assert_eq!((location.line, location.column), (None, None));
    let handle = load(
        sample_with_missing_column().as_bytes(),
        catalog.as_bytes(),
        None,
        &LoadOptions::default(),
    )
    .unwrap();
    let codes: Vec<&str> = handle.warnings().map(|w| w.code.as_str()).collect();
    assert_eq!(codes, ["WCAT-05", "W0D-01"]);
}

#[test]
fn con_0_5_wcat_05_is_reported_with_the_errors_of_a_model_that_does_not_load() {
    let catalog = with_former_name(SAMPLE_CATALOG, "catalog_schema");
    let report = validate(b"version: [", catalog.as_bytes(), &LoadOptions::default());
    assert!(!report.errors.is_empty());
    assert_eq!(report.warnings[0].code, "WCAT-05");
}

#[test]
fn con_0_5_a_rejected_former_name_schema_has_no_wcat_05() {
    let catalog = with_former_name(SAMPLE_CATALOG, "catalog_schema").replacen(
        "\"relations\"",
        "\"relation_list\"",
        1,
    );
    let report = validate(
        SAMPLE_MODEL.as_bytes(),
        catalog.as_bytes(),
        &LoadOptions::default(),
    );
    assert_eq!(report.errors[0].code, "ECAT-01");
    assert!(report.warnings.iter().all(|w| w.code != "WCAT-05"));
}

#[test]
fn con_0_5_wcat_05_a_former_name_state_is_admitted_and_warns_after_the_load_warnings() {
    let handle = load(
        STATE_MODEL.as_bytes(),
        STATE_CATALOG.as_bytes(),
        None,
        &LoadOptions::default(),
    )
    .unwrap();
    let load_warnings = handle.warnings().count();
    let state = with_former_name(STATE, "catalog_state");
    let with_former = with_state(&handle, state.as_bytes());
    assert!(with_former.catalog_state().is_some());
    let warnings: Vec<_> = with_former.warnings().collect();
    assert_eq!(warnings.len(), load_warnings + 1);
    let last = warnings[load_warnings];
    assert_eq!(last.code, "WCAT-05");
    let location = last.location.as_ref().unwrap();
    assert_eq!(location.document, InputDocument::CatalogState);
    assert_eq!(location.pointer, "/contract");
    let with_name = with_state(&handle, STATE.as_bytes());
    assert!(with_name.catalog_state().is_some());
    assert_eq!(with_name.warnings().count(), load_warnings);
}

#[test]
fn con_0_5_an_ignored_former_name_state_reports_only_its_own_code() {
    let handle = load(
        STATE_MODEL.as_bytes(),
        STATE_CATALOG.as_bytes(),
        None,
        &LoadOptions::default(),
    )
    .unwrap();
    let state = with_former_name(STATE, "catalog_state").replacen(
        "\"schema_document_id\": {",
        "\"schema_document_id\": {\"snapshot\": \"other\", \"ignored\": 1, ",
        1,
    );
    let ignored = with_state(&handle, state.as_bytes());
    assert!(ignored.catalog_state().is_none());
    let codes: Vec<&str> = ignored.warnings().map(|w| w.code.as_str()).collect();
    assert!(!codes.contains(&"WCAT-05"), "{codes:?}");
    assert!(codes.iter().any(|code| code.starts_with("WCAT-0")));
}

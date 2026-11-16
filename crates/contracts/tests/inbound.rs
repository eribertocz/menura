//! Admission of incoming documents: A3, CAT-3, QRY-2 and SEC-2.

// `cfg(test)` marks the helpers as test code, where `expect` is allowed (clippy.toml).
#[cfg(test)]
mod support;

use semantic_compiler_contracts::types::catalog::{CatalogSchemaDocument, CatalogStateDocument};
use semantic_compiler_contracts::types::query::QueryDocument;
use semantic_compiler_contracts::types::security_context::SecurityContextDocument;
use semantic_compiler_contracts::{
    AdmissionCode, ContractKind, IncomingContract, check_query_form, decode,
};
use serde_json::{Value, json};
use support::{adding, assert_invalid_at, assert_valid, example, examples, with, without};

/// Runs the admission steps in order: contract and version, then the schema.
fn admit(contract: IncomingContract, document: &Value) -> Result<(), AdmissionCode> {
    contract
        .check_contract(document)
        .map_err(|error| error.code)?;
    contract.validate(document).map_err(|error| error.code)
}

#[test]
fn a3_every_incoming_example_is_admitted_and_decoded() {
    let incoming = [
        IncomingContract::CatalogSchema,
        IncomingContract::CatalogState,
        IncomingContract::Query,
        IncomingContract::SecurityContext,
    ];
    for example in examples() {
        let Some(contract) = incoming
            .into_iter()
            .find(|contract| contract.kind() == example.kind)
        else {
            continue;
        };
        let document = contract.parse_json(example.text).expect("example is JSON");
        assert_eq!(admit(contract, &document), Ok(()), "{}", example.file);
        let decoded = match contract {
            IncomingContract::CatalogSchema => {
                decode::<CatalogSchemaDocument>(contract, &document).map(|_| ())
            }
            IncomingContract::CatalogState => {
                decode::<CatalogStateDocument>(contract, &document).map(|_| ())
            }
            IncomingContract::Query => decode::<QueryDocument>(contract, &document).map(|_| ()),
            IncomingContract::SecurityContext => {
                decode::<SecurityContextDocument>(contract, &document).map(|_| ())
            }
        };
        assert_eq!(decoded, Ok(()), "{}", example.file);
    }
}

#[test]
fn a3_incoming_documents_are_validated_beyond_their_rust_shape() {
    // An empty time zone has the Rust shape of a query but violates its schema (`minLength`).
    let query = with(
        example("query.sql-form.example.json"),
        "/time_zone",
        json!(""),
    );
    assert!(serde_json::from_value::<QueryDocument>(query.clone()).is_ok());
    let error = IncomingContract::Query
        .validate(&query)
        .expect_err("schema rejects");
    assert_eq!(error.code, AdmissionCode::InvalidQuery);
    assert_eq!(error.pointer, "/time_zone");
}

#[test]
fn a3_decode_reads_integral_numbers_as_integers() {
    let document = with(
        example("catalog-state.example.json"),
        "/relations/0/row_count_estimate",
        json!(10.0),
    );
    assert_valid(ContractKind::CatalogState, &document);
    let state: CatalogStateDocument =
        decode(IncomingContract::CatalogState, &document).expect("decodes");
    assert_eq!(state.relations[0].row_count_estimate, 10);
}

#[test]
fn cat_3_catalog_schema_errors() {
    let contract = IncomingContract::CatalogSchema;
    let document = example("catalog-schema.example.json");
    assert_eq!(admit(contract, &document), Ok(()));
    assert_eq!(
        admit(
            contract,
            &with(document.clone(), "/contract_version", json!("1.0.9"))
        ),
        Ok(())
    );
    for version in ["2.0.0", "1.1.0", "0.9.0", "1.0"] {
        let unsupported = with(document.clone(), "/contract_version", json!(version));
        assert_eq!(
            admit(contract, &unsupported),
            Err(AdmissionCode::Ecat02),
            "{version}"
        );
    }
    let wrong_contract = with(
        document.clone(),
        "/contract",
        json!("semantic_compiler.catalog_state"),
    );
    assert_eq!(admit(contract, &wrong_contract), Err(AdmissionCode::Ecat02));
    assert_eq!(
        admit(contract, &without(document.clone(), "/relations")),
        Err(AdmissionCode::Ecat01)
    );
    let bad_precision = with(
        document,
        "/relations/0/columns/5/logical_type/precision",
        json!(39),
    );
    assert_eq!(admit(contract, &bad_precision), Err(AdmissionCode::Ecat01));
    let not_json = contract.parse_json("{\"contract\":").expect_err("not JSON");
    assert_eq!(not_json.code, AdmissionCode::Ecat01);
    assert_eq!(not_json.line_column, Some((1, 12)));
}

#[test]
fn cat_3_catalog_state_errors() {
    let contract = IncomingContract::CatalogState;
    let document = example("catalog-state.example.json");
    assert_eq!(admit(contract, &document), Ok(()));
    let unsupported = with(document.clone(), "/contract_version", json!("2.0.0"));
    assert_eq!(admit(contract, &unsupported), Err(AdmissionCode::Wcat02));
    let two_kinds = adding(
        document.clone(),
        "/relations/0",
        "version",
        json!({"origin": "o", "snapshot": "s"}),
    );
    assert_eq!(admit(contract, &two_kinds), Err(AdmissionCode::Wcat01));
    assert!(serde_json::from_value::<CatalogStateDocument>(two_kinds).is_err());
    let none = without(document.clone(), "/relations/1/version");
    assert_eq!(admit(contract, &none), Err(AdmissionCode::Wcat01));
    let bad_period = with(
        document,
        "/relations/0/partitions/versions/0/period",
        json!("2026-13"),
    );
    let error = contract.validate(&bad_period).expect_err("invalid period");
    assert_eq!(error.code, AdmissionCode::Wcat01);
    assert_eq!(error.pointer, "/relations/0/partitions/versions/0/period");
}

#[test]
fn qry_2_envelope_and_schema_errors() {
    let contract = IncomingContract::Query;
    let document = example("query.json-form.example.json");
    assert_eq!(admit(contract, &document), Ok(()));
    let wrong = with(
        document.clone(),
        "/contract",
        json!("semantic_compiler.query_v2"),
    );
    let error = contract.check_contract(&wrong).expect_err("unsupported");
    assert_eq!(
        (error.code, error.pointer.as_str()),
        (AdmissionCode::Eqry08, "/contract")
    );
    let missing = without(document.clone(), "/contract_version");
    let error = contract.check_contract(&missing).expect_err("unsupported");
    assert_eq!(
        (error.code, error.pointer.as_str()),
        (AdmissionCode::Eqry08, "/contract_version")
    );
    let both = adding(document.clone(), "", "sql", json!("SELECT 1"));
    assert_eq!(
        check_query_form(&both).map_err(|error| error.code),
        Err(AdmissionCode::Eqry01)
    );
    let neither = without(document.clone(), "/query");
    assert_eq!(
        check_query_form(&neither).map_err(|error| error.code),
        Err(AdmissionCode::Eqry01)
    );
    assert_eq!(
        check_query_form(&document).map_err(|error| error.code),
        Ok(())
    );
    let no_zone = without(document.clone(), "/time_zone");
    assert_eq!(admit(contract, &no_zone), Err(AdmissionCode::InvalidQuery));
    let bad_time = with(
        document.clone(),
        "/reference_time",
        json!("2026-11-15T12:00:00"),
    );
    assert_eq!(admit(contract, &bad_time), Err(AdmissionCode::InvalidQuery));
    let null_metric = with(document, "/query/metrics/0", Value::Null);
    let error = contract.validate(&null_metric).expect_err("null item");
    assert_eq!(error.pointer, "/query/metrics/0");
    assert_eq!(
        contract.parse_json("[").map_err(|error| error.code),
        Err(AdmissionCode::InvalidQuery)
    );
}

#[test]
fn qry_2_extension_rules_of_the_schema() {
    let base = example("query.json-form.example.json");
    let range = "/query/nonportable_extensions/0/data/date_range";
    let absolute_and_relative = adding(base.clone(), range, "from", json!("2026-01-01"));
    assert_invalid_at(ContractKind::Query, &absolute_and_relative, range);
    let absolute = with(
        base.clone(),
        range,
        json!({"dimension": "invoices.invoice_date", "from": "2026-01-01", "to": "2026-02-30"}),
    );
    assert_invalid_at(ContractKind::Query, &absolute, &format!("{range}/to"));
    let namespace = with(
        base,
        "/query/nonportable_extensions/0/data/namespace",
        json!("acme"),
    );
    assert_invalid_at(
        ContractKind::Query,
        &namespace,
        "/query/nonportable_extensions/0/data/namespace",
    );
}

#[test]
fn sec_2_security_context_errors() {
    let contract = IncomingContract::SecurityContext;
    let document = example("security-context.example.json");
    assert_eq!(admit(contract, &document), Ok(()));
    let unsupported = with(document.clone(), "/contract_version", json!("1.1.0"));
    assert_eq!(admit(contract, &unsupported), Err(AdmissionCode::Esec03));
    assert_eq!(
        admit(contract, &without(document.clone(), "/roles")),
        Err(AdmissionCode::Esec01)
    );
    let duplicate = with(document.clone(), "/roles", json!(["analyst", "analyst"]));
    assert_eq!(
        contract.validate(&duplicate).map_err(|error| error.pointer),
        Err("/roles/1".to_owned())
    );
    let unsafe_integer = with(
        document.clone(),
        "/attributes/regions",
        json!(9_007_199_254_740_992_u64),
    );
    assert_eq!(admit(contract, &unsafe_integer), Err(AdmissionCode::Esec01));
    let nested = with(document.clone(), "/attributes/regions", json!([["NORTH"]]));
    assert_eq!(admit(contract, &nested), Err(AdmissionCode::Esec01));
    let long_subject = with(document, "/subject", json!("x".repeat(257)));
    assert_eq!(admit(contract, &long_subject), Err(AdmissionCode::Esec01));
}

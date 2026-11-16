//! Contract document tests: CON-0.1, CON-0.2, CON-1, PLN-1, PLN-5, CHK-2 and DSC-2.

// `cfg(test)` marks the helpers as test code, where `expect` is allowed (clippy.toml).
#[cfg(test)]
mod support;

use semantic_compiler_contracts::ContractKind;
use semantic_compiler_contracts::types::ContractDocument;
use semantic_compiler_contracts::types::audit_record::AuditRecordDocument;
use semantic_compiler_contracts::types::catalog::{CatalogSchemaDocument, CatalogStateDocument};
use semantic_compiler_contracts::types::compiled_plan::{
    CompileErrorDocument, CompiledPlanDocument,
};
use semantic_compiler_contracts::types::descriptions::{
    IntegrityChecksDocument, ModelDescriptionDocument,
};
use semantic_compiler_contracts::types::query::QueryDocument;
use semantic_compiler_contracts::types::security_context::SecurityContextDocument;
use serde_json::{Value, json};
use support::{adding, assert_invalid_at, assert_valid, example, examples, with, without};

/// Decodes into the Rust type and encodes back; `None` when either step fails.
fn round_trip<T: ContractDocument>(document: &Value) -> Option<Value> {
    let decoded: T = serde_json::from_value(document.clone()).ok()?;
    serde_json::to_value(decoded).ok()
}

fn round_trip_of(kind: ContractKind, document: &Value) -> Option<Value> {
    match kind {
        ContractKind::CatalogSchema => round_trip::<CatalogSchemaDocument>(document),
        ContractKind::CatalogState => round_trip::<CatalogStateDocument>(document),
        ContractKind::Query => round_trip::<QueryDocument>(document),
        ContractKind::SecurityContext => round_trip::<SecurityContextDocument>(document),
        ContractKind::CompiledPlan => round_trip::<CompiledPlanDocument>(document),
        ContractKind::CompileError => round_trip::<CompileErrorDocument>(document),
        ContractKind::AuditRecord => round_trip::<AuditRecordDocument>(document),
        ContractKind::ModelDescription => round_trip::<ModelDescriptionDocument>(document),
        ContractKind::IntegrityChecks => round_trip::<IntegrityChecksDocument>(document),
    }
}

#[test]
fn con_0_1_every_public_contract_has_a_kind() {
    let readme = include_str!("../../../spec/contracts/README.md");
    let listed: Vec<&str> = readme
        .lines()
        .filter_map(|line| line.split('`').nth(1))
        .filter(|name| name.starts_with("semantic_compiler."))
        .collect();
    let kinds: Vec<&str> = ContractKind::ALL
        .iter()
        .map(|kind| kind.contract_name())
        .collect();
    assert_eq!(listed, kinds);
}

#[test]
fn con_0_2_every_contract_has_a_valid_example_and_a_versioned_schema_id() {
    for kind in ContractKind::ALL {
        assert!(
            examples().iter().any(|example| example.kind == kind),
            "{kind} has an example"
        );
    }
    for example in examples() {
        let document: Value = serde_json::from_str(example.text).expect("example is JSON");
        assert_valid(example.kind, &document);
        assert_eq!(
            document["contract_version"].as_str(),
            Some(example.kind.latest_version().to_string().as_str()),
            "{}",
            example.file
        );
    }
}

#[test]
fn con_0_2_every_example_round_trips_unchanged() {
    for example in examples() {
        let document: Value = serde_json::from_str(example.text).expect("example is JSON");
        assert_eq!(
            round_trip_of(example.kind, &document),
            Some(document.clone()),
            "{}",
            example.file
        );
    }
}

#[test]
fn con_1_every_object_is_closed() {
    for example in examples() {
        let document: Value = serde_json::from_str(example.text).expect("example is JSON");
        let open = adding(document, "", "unknown_member", json!(1));
        assert_invalid_at(example.kind, &open, "/unknown_member");
    }
    let column = adding(
        example("catalog-schema.example.json"),
        "/relations/0/columns/0",
        "extra",
        json!(true),
    );
    assert_invalid_at(
        ContractKind::CatalogSchema,
        &column,
        "/relations/0/columns/0/extra",
    );
    let logical_type = adding(
        example("catalog-schema.example.json"),
        "/relations/0/columns/1/logical_type",
        "bits",
        json!(8),
    );
    assert_invalid_at(
        ContractKind::CatalogSchema,
        &logical_type,
        "/relations/0/columns/1/logical_type",
    );
}

#[test]
fn con_1_typed_documents_reject_unknown_members() {
    let open = adding(
        example("security-context.example.json"),
        "",
        "extra",
        json!(1),
    );
    assert!(serde_json::from_value::<SecurityContextDocument>(open).is_err());
    let nested = adding(
        example("query.json-form.example.json"),
        "/query",
        "extra",
        json!(1),
    );
    assert!(serde_json::from_value::<QueryDocument>(nested).is_err());
}

#[test]
fn pln_1_compiled_plan_schema() {
    let kind = ContractKind::CompiledPlan;
    let document = example("compiled-plan.example.json");
    assert_invalid_at(
        kind,
        &with(document.clone(), "/result_schema", json!([])),
        "/result_schema",
    );
    assert_invalid_at(kind, &with(document.clone(), "/sql", json!("")), "/sql");
    assert_invalid_at(
        kind,
        &with(document.clone(), "/dialect", json!("postgres")),
        "/dialect",
    );
    assert_invalid_at(
        kind,
        &with(document.clone(), "/cache_key/key", json!("AA")),
        "/cache_key/key",
    );
    assert_valid(
        kind,
        &with(document.clone(), "/cache_key/data_versions", Value::Null),
    );
    let explanation = with(
        document.clone(),
        "/contract",
        json!("semantic_compiler.explanation"),
    );
    assert_invalid_at(kind, &explanation, "/parameters");
    let explanation = with(
        with(explanation, "/sql", json!("")),
        "/parameters",
        json!([]),
    );
    assert_valid(kind, &explanation);
    let parameter = with(document, "/parameters/0/position", json!(0));
    assert_invalid_at(kind, &parameter, "/parameters/0/position");
}

#[test]
fn pln_5_compile_error_schema() {
    let kind = ContractKind::CompileError;
    let document = example("compile-error.example.json");
    assert_invalid_at(
        kind,
        &with(document.clone(), "/errors", json!([])),
        "/errors",
    );
    let report = with(
        document.clone(),
        "/contract",
        json!("semantic_compiler.validation_report"),
    );
    assert_valid(kind, &with(report, "/errors", json!([])));
    assert_invalid_at(
        kind,
        &with(document.clone(), "/errors/0/code", json!("e_bad")),
        "/errors/0/code",
    );
    assert_valid(
        kind,
        &with(document.clone(), "/errors/0/location", Value::Null),
    );
    let located = adding(document.clone(), "/errors/0/location", "line", json!(0));
    assert_invalid_at(kind, &located, "/errors/0/location/line");
    let null_location = with(document, "/errors/0/location", Value::Null);
    assert_eq!(
        round_trip::<CompileErrorDocument>(&null_location),
        Some(null_location)
    );
}

#[test]
fn chk_2_integrity_checks_schema() {
    let kind = ContractKind::IntegrityChecks;
    let document = example("integrity-checks.example.json");
    assert_invalid_at(
        kind,
        &with(document.clone(), "/checks/0/kind", json!("freshness")),
        "/checks/0/kind",
    );
    assert_invalid_at(
        kind,
        &with(document.clone(), "/checks/0/sources", json!([])),
        "/checks/0/sources",
    );
    assert_invalid_at(
        kind,
        &without(document, "/checks/0/definition_hash"),
        "/checks/0",
    );
}

#[test]
fn dsc_2_model_description_schema() {
    let kind = ContractKind::ModelDescription;
    let document = example("model-description.example.json");
    assert_invalid_at(kind, &without(document.clone(), "/metrics"), "");
    let grain = with(
        document.clone(),
        "/datasets/1/fields/4/time_grains/0",
        json!("hour"),
    );
    assert_invalid_at(kind, &grain, "/datasets/1/fields/4/time_grains/0");
    assert_invalid_at(
        kind,
        &with(document.clone(), "/metrics/0/kind", json!("ratio")),
        "/metrics/0/kind",
    );
    let context = adding(document, "/metrics/0", "ai_context", Value::Null);
    assert_valid(kind, &context);
    assert_eq!(
        round_trip::<ModelDescriptionDocument>(&context),
        Some(context)
    );
}

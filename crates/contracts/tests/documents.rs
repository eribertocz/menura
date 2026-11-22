//! Contract document tests: CON-0.1, CON-0.2, CON-1, PLN-5 and DSC-2. The schema verdicts of
//! each contract are the cases of `spec/contracts/corpus/` (`tests/corpus.rs`).

// `cfg(test)` marks the helpers as test code, where `expect` is allowed (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::ContractDocument;
use menura_contracts::types::audit_record::AuditRecordDocument;
use menura_contracts::types::catalog::{CatalogSchemaDocument, CatalogStateDocument};
use menura_contracts::types::compiled_plan::{CompileErrorDocument, CompiledPlanDocument};
use menura_contracts::types::dependencies::{ModelDependenciesDocument, ModelDiffDocument};
use menura_contracts::types::descriptions::{IntegrityChecksDocument, ModelDescriptionDocument};
use menura_contracts::types::query::QueryDocument;
use menura_contracts::types::security_context::SecurityContextDocument;
use menura_contracts::{ContractKind, ContractVersion, FORMER_NAMESPACE, NAMESPACE};
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
        ContractKind::ModelDependencies => round_trip::<ModelDependenciesDocument>(document),
        ContractKind::ModelDiff => round_trip::<ModelDiffDocument>(document),
    }
}

#[test]
fn con_0_1_every_public_contract_has_a_kind() {
    let readme = include_str!("../../../spec/contracts/README.md");
    // Both tables (the contracts produced and those planned for 0.8.0) name each contract with
    // its name or its former name (CON-0.5), in whichever column holds the `contract` value, so
    // every code span of every table row is read and only the part after the namespace is
    // compared.
    let listed: Vec<&str> = readme
        .lines()
        .filter(|line| line.starts_with('|'))
        .flat_map(|line| line.split('`').skip(1).step_by(2))
        .filter_map(|name| name.split_once('.'))
        .filter(|(namespace, _)| [NAMESPACE, FORMER_NAMESPACE].contains(namespace))
        .map(|(_, local_name)| local_name)
        .collect();
    let kinds: Vec<&str> = ContractKind::ALL
        .iter()
        .map(|kind| kind.local_name())
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
        // An example may still carry a previous minor version (CON-0.5); it must be accepted.
        let version = document["contract_version"]
            .as_str()
            .and_then(ContractVersion::parse)
            .expect("the example has a contract version");
        assert!(
            example.kind.accepts_version(version),
            "{}: version {version} is accepted",
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
fn pln_5_a_null_location_round_trips() {
    let null_location = with(
        example("compile-error.example.json"),
        "/errors/0/location",
        Value::Null,
    );
    assert_eq!(
        round_trip::<CompileErrorDocument>(&null_location),
        Some(null_location)
    );
}

#[test]
fn dsc_2_a_null_ai_context_round_trips() {
    let context = adding(
        example("model-description.example.json"),
        "/metrics/0",
        "ai_context",
        Value::Null,
    );
    assert_eq!(
        round_trip::<ModelDescriptionDocument>(&context),
        Some(context)
    );
}

#[test]
fn pln_1_a_typed_plan_requires_every_member() {
    let incomplete = without(example("compiled-plan.example.json"), "/lineage");
    assert!(serde_json::from_value::<CompiledPlanDocument>(incomplete).is_err());
}

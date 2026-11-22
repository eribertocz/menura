//! Contract versions (CON-3) and the optional members contract version 1.2.0 adds to the types of
//! the compiled plan (PLN-1, PLN-4, PLN-5.4), the audit record (PLN-6), the model description
//! (DSC-2, DSC-3.6) and the integrity checks (CHK-2, CHK-7).
//!
//! The members are optional, so these tests decode documents built from the published examples,
//! whatever their contract version; the schema verdicts are the cases of `spec/contracts/corpus/`.

// `cfg(test)` marks the helpers as test code, where `expect` is allowed (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::audit_record::AuditRecordDocument;
use menura_contracts::types::common::Step;
use menura_contracts::types::compiled_plan::{CompileErrorDocument, CompiledPlanDocument};
use menura_contracts::types::descriptions::{
    AiContext, AiContextTrust, Binding, CheckParameter, IntegrityChecksDocument,
    ModelDescriptionDocument,
};
use menura_contracts::{ContractKind, ContractVersion};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use support::{adding, assert_invalid_at, assert_valid, example, examples, with, without};

/// The published JSON Schema text of a contract.
fn schema_text(kind: ContractKind) -> &'static str {
    match kind {
        ContractKind::CatalogSchema => {
            include_str!("../../../spec/contracts/schemas/catalog-schema.schema.json")
        }
        ContractKind::CatalogState => {
            include_str!("../../../spec/contracts/schemas/catalog-state.schema.json")
        }
        ContractKind::Query => include_str!("../../../spec/contracts/schemas/query.schema.json"),
        ContractKind::SecurityContext => {
            include_str!("../../../spec/contracts/schemas/security-context.schema.json")
        }
        ContractKind::CompiledPlan => {
            include_str!("../../../spec/contracts/schemas/compiled-plan.schema.json")
        }
        ContractKind::CompileError => {
            include_str!("../../../spec/contracts/schemas/compile-error.schema.json")
        }
        ContractKind::AuditRecord => {
            include_str!("../../../spec/contracts/schemas/audit-record.schema.json")
        }
        ContractKind::ModelDescription => {
            include_str!("../../../spec/contracts/schemas/model-description.schema.json")
        }
        ContractKind::IntegrityChecks => {
            include_str!("../../../spec/contracts/schemas/integrity-checks.schema.json")
        }
    }
}

// `cfg(test)` marks the helpers as test code, where `expect` is allowed (clippy.toml).
#[cfg(test)]
fn decode<T: DeserializeOwned>(document: &Value) -> T {
    serde_json::from_value(document.clone()).expect("the document decodes")
}

#[cfg(test)]
fn round_trips<T: DeserializeOwned + serde::Serialize>(document: &Value) -> bool {
    let decoded: T = decode(document);
    serde_json::to_value(decoded).ok().as_ref() == Some(document)
}

#[cfg(test)]
fn version(text: &str) -> ContractVersion {
    ContractVersion::parse(text).expect("a semantic version")
}

#[test]
fn con_3_the_latest_version_is_the_version_of_the_schema_id() {
    for kind in ContractKind::ALL {
        let schema: Value = serde_json::from_str(schema_text(kind)).expect("schema is JSON");
        let identifier = schema["$id"].as_str().expect("the schema has an $id");
        let expected = identifier.rsplit('/').next().map(version);
        assert_eq!(Some(kind.latest_version()), expected, "{kind}");
    }
}

#[test]
fn con_3_every_example_is_at_its_contract_latest_version() {
    for example in examples() {
        let document: Value = serde_json::from_str(example.text).expect("example is JSON");
        assert_eq!(
            document["contract_version"].as_str().map(version),
            Some(example.kind.latest_version()),
            "{}",
            example.file
        );
        assert_valid(example.kind, &document);
        // Every contract requires its version (CON-3).
        assert_invalid_at(example.kind, &without(document, "/contract_version"), "");
    }
}

#[test]
fn con_3_earlier_minor_versions_stay_accepted_and_later_ones_are_not() {
    for kind in ContractKind::ALL {
        let latest = kind.latest_version();
        assert!(kind.accepts_version(version("1.0.0")), "{kind}");
        assert!(kind.accepts_version(latest), "{kind}");
        let next_minor = ContractVersion {
            minor: latest.minor + 1,
            ..latest
        };
        assert!(!kind.accepts_version(next_minor), "{kind}");
        let next_major = ContractVersion {
            major: latest.major + 1,
            minor: 0,
            patch: 0,
        };
        assert!(!kind.accepts_version(next_major), "{kind}");
    }
}

#[test]
fn pln_1_explanation_entries_may_carry_a_code_and_a_subject() {
    let entry = json!({
        "code": "DEV-4",
        "step": "E0",
        "subject": "load_option:grant_authenticated_to_all",
        "decision": "datasets without access are inaccessible",
        "reason": "grant_authenticated_to_all is false"
    });
    let document = with(
        example("compiled-plan.example.json"),
        "/explanation",
        json!([entry]),
    );
    let plan: CompiledPlanDocument = decode(&document);
    assert_eq!(plan.explanation[0].code.as_deref(), Some("DEV-4"));
    assert_eq!(plan.explanation[0].step, Step::E0);
    assert!(round_trips::<CompiledPlanDocument>(&document));
    let without_code = with(
        document,
        "/explanation",
        json!([{"step": "E2", "decision": "d", "reason": "r"}]),
    );
    let plan: CompiledPlanDocument = decode(&without_code);
    assert_eq!(plan.explanation[0].subject, None);
    assert!(round_trips::<CompiledPlanDocument>(&without_code));
}

#[test]
fn pln_4_lineage_has_one_entry_per_result_column_in_the_example() {
    let plan: CompiledPlanDocument = decode(&example("compiled-plan.example.json"));
    let outputs: Vec<&str> = plan
        .lineage
        .iter()
        .map(|entry| entry.output.as_str())
        .collect();
    let columns: Vec<&str> = plan
        .result_schema
        .iter()
        .map(|column| column.name.as_str())
        .collect();
    assert_eq!(outputs, columns);
}

#[test]
fn pln_5_4_a_diagnostic_may_carry_alternatives() {
    let document = adding(
        example("compile-error.example.json"),
        "/errors/0",
        "alternatives",
        json!(["metric:billed"]),
    );
    let error: CompileErrorDocument = decode(&document);
    let alternatives = error.errors[0]
        .alternatives
        .as_deref()
        .map(|alternatives| alternatives.0.as_slice());
    assert_eq!(alternatives, Some(["metric:billed".to_owned()].as_slice()));
    assert!(round_trips::<CompileErrorDocument>(&document));
    let without_alternatives = without(document, "/errors/0/alternatives");
    let error: CompileErrorDocument = decode(&without_alternatives);
    assert_eq!(error.errors[0].alternatives, None);
}

#[test]
fn pln_6_an_audit_record_may_carry_compile_options_data_source_and_actor() {
    let document = adding(
        example("audit-record.example.json"),
        "",
        "actor",
        json!("agent:report-assistant"),
    );
    let document = adding(
        document,
        "",
        "compile_options",
        json!({"dialect": "duckdb"}),
    );
    let document = adding(
        document,
        "/load_options",
        "data_source",
        json!("warehouse:gold"),
    );
    let record: AuditRecordDocument = decode(&document);
    assert_eq!(record.actor.as_deref(), Some("agent:report-assistant"));
    assert!(record.compile_options.is_some());
    assert_eq!(
        record.load_options.data_source.as_deref(),
        Some("warehouse:gold")
    );
    assert!(round_trips::<AuditRecordDocument>(&document));
}

#[test]
fn pln_6_4_a_context_not_admitted_has_a_null_subject_and_fingerprint() {
    let document = with(
        example("audit-record.example.json"),
        "/subject",
        Value::Null,
    );
    let document = with(document, "/inputs/security_fingerprint", Value::Null);
    let record: AuditRecordDocument = decode(&document);
    assert_eq!(record.subject, None);
    assert_eq!(record.inputs.security_fingerprint, None);
    assert!(round_trips::<AuditRecordDocument>(&document));
}

#[test]
fn dsc_2_a_description_may_carry_the_capability_profile_and_metric_labels() {
    let capabilities = json!({
        "compiler_version": "0.7.0",
        "slice_version": "0.7.0",
        "ossie": {"normative": "0.1.1", "tracked_commit": "aa500d3", "formats": ["release"]},
        "contracts": [{"contract": "menura.query", "version": "1.1.0"}],
        "dialects": ["duckdb"],
        "query_forms": ["sql", "wide_table"],
        "extensions": [{"key": "access", "versions": [1]}],
        "functions": ["compile", "describe"],
        "limits": {"max_identifier_length": 128, "max_decimal_precision": 38}
    });
    let document = adding(
        example("model-description.example.json"),
        "",
        "capabilities",
        capabilities,
    );
    let document = adding(document, "/metrics/0", "label", json!("Average invoice"));
    let description: ModelDescriptionDocument = decode(&document);
    let profile = description.capabilities.expect("the profile decodes");
    assert_eq!(profile.slice_version, version("0.7.0"));
    assert_eq!(
        description.metrics[0].label.as_deref(),
        Some("Average invoice")
    );
    assert!(round_trips::<ModelDescriptionDocument>(&document));
}

#[test]
fn dsc_3_6_ai_context_decodes_with_or_without_its_provenance() {
    let attributed = json!({
        "value": "One row per customer.",
        "source": {
            "pointer": "/semantic_model/0/datasets/1/ai_context",
            "model_hash": "1111111111111111111111111111111111111111111111111111111111111111"
        },
        "trust": "untrusted_model_content"
    });
    let document = adding(
        example("model-description.example.json"),
        "/datasets/0",
        "ai_context",
        attributed,
    );
    let description: ModelDescriptionDocument = decode(&document);
    let Some(AiContext::Attributed(context)) = &description.datasets[0].ai_context else {
        panic!("the dataset ai_context has its provenance");
    };
    assert_eq!(context.trust, AiContextTrust::UntrustedModelContent);
    assert!(round_trips::<ModelDescriptionDocument>(&document));
    let unattributed = with(document, "/datasets/0/ai_context", json!({"synonyms": []}));
    let description: ModelDescriptionDocument = decode(&unattributed);
    assert_eq!(
        description.datasets[0].ai_context,
        Some(AiContext::Unattributed(json!({"synonyms": []})))
    );
    assert!(round_trips::<ModelDescriptionDocument>(&unattributed));
}

#[test]
fn chk_2_a_check_may_carry_its_build_query() {
    let build = json!({
        "sql": "SELECT 1 AS \"x\"",
        "parameters": [],
        "columns": ["x"]
    });
    let document = adding(
        example("integrity-checks.example.json"),
        "/checks/0",
        "build",
        build,
    );
    let checks: IntegrityChecksDocument = decode(&document);
    let build = checks.checks[0].build.as_ref().expect("the build decodes");
    assert_eq!(build.columns, ["x"]);
    assert!(round_trips::<IntegrityChecksDocument>(&document));
}

#[test]
fn chk_7_a_parameter_has_a_value_or_a_text_binding() {
    let slot = json!({"position": 1, "logical_type": {"kind": "text"}, "binding": "period_from"});
    let document = with(
        example("integrity-checks.example.json"),
        "/checks/0/parameters",
        json!([slot]),
    );
    let checks: IntegrityChecksDocument = decode(&document);
    let Some(CheckParameter::Bound(bound)) = checks.checks[0].parameters.first() else {
        panic!("the parameter is a slot");
    };
    assert_eq!(bound.binding, Binding::PeriodFrom);
    assert!(round_trips::<IntegrityChecksDocument>(&document));
    let fixed = with(
        document.clone(),
        "/checks/0/parameters/0",
        json!({"position": 1, "logical_type": {"kind": "text"}, "value": "UTC"}),
    );
    let checks: IntegrityChecksDocument = decode(&fixed);
    assert!(matches!(
        checks.checks[0].parameters.first(),
        Some(CheckParameter::Fixed(_))
    ));
    let both = adding(
        document.clone(),
        "/checks/0/parameters/0",
        "value",
        json!("2024-01"),
    );
    assert!(serde_json::from_value::<IntegrityChecksDocument>(both).is_err());
    let dated = with(
        document,
        "/checks/0/parameters/0/logical_type",
        json!({"kind": "date"}),
    );
    assert!(serde_json::from_value::<IntegrityChecksDocument>(dated).is_err());
}

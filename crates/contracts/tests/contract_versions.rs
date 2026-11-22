//! The 1.2.0 contracts (CON-3): per-contract latest versions, and the members 1.2.0 adds to the
//! compiled plan (PLN-1, PLN-4, PLN-5.4), the audit record (PLN-6), the model description
//! (DSC-2, DSC-3.6) and the integrity checks (CHK-2, CHK-7).

// `cfg(test)` marks the helpers as test code, where `expect` is allowed (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::audit_record::AuditRecordDocument;
use menura_contracts::types::common::Step;
use menura_contracts::types::compiled_plan::{CompileErrorDocument, CompiledPlanDocument};
use menura_contracts::types::descriptions::{
    AiContext, AiContextTrust, Binding, CheckKind, CheckParameter, IntegrityChecksDocument,
    ModelDescriptionDocument,
};
use menura_contracts::{ContractKind, ContractVersion};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use support::{adding, assert_invalid_at, assert_valid, example, examples, with, without};

// `cfg(test)` marks the helpers as test code, where `expect` is allowed (clippy.toml).
#[cfg(test)]
fn decode<T: DeserializeOwned>(document: &Value) -> T {
    serde_json::from_value(document.clone()).expect("the document decodes")
}

#[cfg(test)]
fn version(text: &str) -> ContractVersion {
    ContractVersion::parse(text).expect("a semantic version")
}

#[test]
fn con_3_plan_outputs_are_at_1_2_0_and_inputs_at_1_1_0() {
    let at_1_2_0 = [
        ContractKind::CompiledPlan,
        ContractKind::CompileError,
        ContractKind::AuditRecord,
        ContractKind::ModelDescription,
        ContractKind::IntegrityChecks,
    ];
    for kind in ContractKind::ALL {
        let expected = if at_1_2_0.contains(&kind) {
            "1.2.0"
        } else {
            "1.1.0"
        };
        assert_eq!(kind.latest_version().to_string(), expected, "{kind}");
    }
    for example in examples() {
        let document: Value = serde_json::from_str(example.text).expect("example is JSON");
        assert_eq!(
            document["contract_version"].as_str(),
            Some(example.kind.latest_version().to_string().as_str()),
            "{}",
            example.file
        );
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
        assert!(!kind.accepts_version(version("2.0.0")), "{kind}");
    }
}

#[test]
fn pln_1_explanation_entries_carry_their_code_subject_and_step() {
    let plan: CompiledPlanDocument = decode(&example("compiled-plan.example.json"));
    let first = &plan.explanation[0];
    assert_eq!(first.code.as_deref(), Some("DEV-4"));
    assert_eq!(first.step, Step::E0);
    assert_eq!(
        first.subject.as_deref(),
        Some("load_option:grant_authenticated_to_all")
    );
    let previous = without(
        without(example("compiled-plan.example.json"), "/explanation/0/code"),
        "/explanation/0/subject",
    );
    let previous = with(previous, "/explanation/0/step", json!("E1"));
    let decoded: CompiledPlanDocument = decode(&previous);
    assert_eq!(decoded.explanation[0].code, None);
    assert_eq!(serde_json::to_value(decoded).ok(), Some(previous));
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
fn pln_5_4_name_not_found_carries_alternatives() {
    let kind = ContractKind::CompileError;
    let document = example("compile-error.example.json");
    let error: CompileErrorDocument = decode(&document);
    assert_eq!(
        error.errors[0]
            .alternatives
            .as_deref()
            .map(|alternatives| alternatives.0.as_slice()),
        Some(["metric:billed".to_owned()].as_slice())
    );
    assert_invalid_at(
        kind,
        &with(document.clone(), "/errors/0/alternatives", json!([])),
        "/errors/0/alternatives",
    );
    assert_invalid_at(
        kind,
        &with(
            document.clone(),
            "/errors/0/alternatives/0",
            json!("relationship:x"),
        ),
        "/errors/0/alternatives/0",
    );
    let without_alternatives = without(document, "/errors/0/alternatives");
    let decoded: CompileErrorDocument = decode(&without_alternatives);
    assert_eq!(decoded.errors[0].alternatives, None);
    assert_eq!(
        serde_json::to_value(decoded).ok(),
        Some(without_alternatives)
    );
}

#[test]
fn pln_6_audit_record_carries_compile_options_data_source_and_actor() {
    let record: AuditRecordDocument = decode(&example("audit-record.example.json"));
    assert_eq!(record.actor.as_deref(), Some("agent:report-assistant"));
    assert!(record.compile_options.is_some());
    assert_eq!(
        record.load_options.data_source.as_deref(),
        Some("warehouse:gold")
    );
    assert_eq!(record.decisions[0].step, Step::E0);
}

#[test]
fn pln_6_4_a_context_not_admitted_has_null_subject_and_fingerprint() {
    let kind = ContractKind::AuditRecord;
    let document = with(
        with(
            example("audit-record.example.json"),
            "/subject",
            Value::Null,
        ),
        "/inputs/security_fingerprint",
        Value::Null,
    );
    let document = with(document, "/outcome", json!({"error_codes": ["ESEC-01"]}));
    assert_valid(kind, &document);
    let record: AuditRecordDocument = decode(&document);
    assert_eq!(record.subject, None);
    assert_eq!(record.inputs.security_fingerprint, None);
    assert_eq!(serde_json::to_value(record).ok(), Some(document));
}

#[test]
fn dsc_2_the_description_carries_the_capability_profile() {
    let description: ModelDescriptionDocument = decode(&example("model-description.example.json"));
    let capabilities = description.capabilities.expect("the example has a profile");
    assert_eq!(capabilities.slice_version, version("0.7.0"));
    assert_eq!(capabilities.dialects, ["duckdb"]);
    assert_invalid_at(
        ContractKind::ModelDescription,
        &without(
            example("model-description.example.json"),
            "/capabilities/limits",
        ),
        "/capabilities",
    );
}

#[test]
fn dsc_3_6_ai_context_carries_its_provenance_from_1_2_0() {
    let kind = ContractKind::ModelDescription;
    let document = example("model-description.example.json");
    let description: ModelDescriptionDocument = decode(&document);
    let Some(AiContext::Attributed(context)) = &description.datasets[0].ai_context else {
        panic!("the example dataset has an attributed ai_context");
    };
    assert_eq!(context.trust, AiContextTrust::UntrustedModelContent);
    assert_eq!(
        context.source.pointer,
        "/semantic_model/0/datasets/1/ai_context"
    );
    let raw = with(document.clone(), "/datasets/0/ai_context", json!("raw"));
    assert_invalid_at(kind, &raw, "/datasets/0/ai_context");
    let previous = with(raw, "/contract_version", json!("1.1.0"));
    assert_valid(kind, &previous);
    let decoded: ModelDescriptionDocument = decode(&previous);
    assert_eq!(
        decoded.datasets[0].ai_context,
        Some(AiContext::Unattributed(json!("raw")))
    );
    assert_eq!(serde_json::to_value(decoded).ok(), Some(previous));
}

#[test]
fn chk_2_every_summary_check_carries_its_build_query() {
    let kind = ContractKind::IntegrityChecks;
    let document = example("integrity-checks.example.json");
    let checks: IntegrityChecksDocument = decode(&document);
    for check in &checks.checks {
        assert_eq!(
            check.build.is_some(),
            check.kind == CheckKind::Summary,
            "{}",
            check.check_id
        );
    }
    assert_invalid_at(
        kind,
        &without(document.clone(), "/checks/1/build"),
        "/checks/1",
    );
    let build = json!({"sql": "SELECT 1", "parameters": [], "columns": ["x"]});
    assert_invalid_at(
        kind,
        &adding(document, "/checks/0", "build", build),
        "/checks/0",
    );
}

#[test]
fn chk_7_a_parameter_has_a_value_or_a_text_binding() {
    let kind = ContractKind::IntegrityChecks;
    let document = example("integrity-checks.example.json");
    let checks: IntegrityChecksDocument = decode(&document);
    let Some(CheckParameter::Bound(slot)) = checks.checks[1].parameters.first() else {
        panic!("the summary check has a slot");
    };
    assert_eq!(slot.binding, Binding::PeriodFrom);
    let fixed = with(
        document.clone(),
        "/checks/1/parameters/0",
        json!({"position": 1, "logical_type": {"kind": "text"}, "value": "UTC"}),
    );
    let decoded: IntegrityChecksDocument = decode(&fixed);
    assert!(matches!(
        decoded.checks[1].parameters.first(),
        Some(CheckParameter::Fixed(_))
    ));
    let both = adding(
        document.clone(),
        "/checks/1/parameters/0",
        "value",
        json!("2024-01"),
    );
    assert_invalid_at(kind, &both, "/checks/1/parameters/0");
    assert!(serde_json::from_value::<IntegrityChecksDocument>(both).is_err());
    let dated = with(
        document,
        "/checks/1/parameters/0/logical_type",
        json!({"kind": "date"}),
    );
    assert_invalid_at(kind, &dated, "/checks/1/parameters/0/logical_type");
    assert!(serde_json::from_value::<IntegrityChecksDocument>(dated).is_err());
}

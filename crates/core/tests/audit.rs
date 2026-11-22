//! The audit record of `compile` with `audit: true` (PLN-6, E2-4.7, VS-1e).

mod support;

use menura_contracts::document_hash;
use menura_contracts::types::audit_record::{Access, AuditRecordDocument, Outcome};
use menura_core::{Compilation, CompileOptions, CompileResult, Handle, LoadOptions, compile};
use support::{
    CATALOG_SCHEMA, MODEL, amount_by_region, analyst, context, error, handle, handle_with, plan,
};

/// The options of an audited compilation, with `actor` when given.
fn audited(actor: Option<&str>) -> CompileOptions {
    CompileOptions {
        audit: true,
        actor: actor.map(str::to_owned),
        ..CompileOptions::default()
    }
}

/// Compiles `query` for `context` over `handle` with `options`.
fn run(handle: &Handle, query: &str, context: &str, options: &CompileOptions) -> Compilation {
    compile(handle, query.as_bytes(), context.as_bytes(), options)
}

/// The audit record of `compilation`; fails the test without one. `cfg(test)` marks it as test
/// code, where `expect` is allowed (clippy.toml).
#[cfg(test)]
fn record(compilation: &Compilation) -> &AuditRecordDocument {
    compilation.audit.as_deref().expect("an audit record")
}

/// An administrator, to whom no row policy applies.
fn admin() -> String {
    context("admin-1", &["admin"], "{}")
}

/// `(object, decision, rule)` of every authorization entry of `record`.
fn entries(record: &AuditRecordDocument) -> Vec<(&str, Access, &str)> {
    record
        .authorization
        .iter()
        .map(|entry| (entry.object.as_str(), entry.decision, entry.rule.as_str()))
        .collect()
}

#[test]
fn pln_6_without_audit_there_is_no_record() {
    let compilation = run(
        &handle(),
        &amount_by_region(),
        &analyst(),
        &CompileOptions::default(),
    );
    assert_eq!(compilation.audit, None);
}

#[test]
fn vs_1e_the_plan_with_audit_is_the_plan_without_it() {
    let without = run(
        &handle(),
        &amount_by_region(),
        &analyst(),
        &CompileOptions::default(),
    );
    let with = run(
        &handle(),
        &amount_by_region(),
        &analyst(),
        &audited(Some("agent:report")),
    );
    assert_eq!(with.result, without.result);
}

#[test]
fn vs_1e_the_error_with_audit_is_the_error_without_it() {
    let query = support::query(r#"{"dimensions": ["orders.colour"]}"#);
    let without = run(&handle(), &query, &analyst(), &CompileOptions::default());
    let with = run(&handle(), &query, &analyst(), &audited(None));
    assert_eq!(with.result, without.result);
    assert!(matches!(with.result, CompileResult::Error(_)));
}

#[test]
fn pln_6_3_the_inputs_are_the_hashes_of_the_documents_as_passed() {
    let handle = handle();
    let compilation = run(&handle, &amount_by_region(), &analyst(), &audited(None));
    let record = record(&compilation);
    let plan = plan(compilation.result.clone());
    assert_eq!(&record.inputs.model, handle.model().hash());
    assert_eq!(
        record.inputs.catalog_schema,
        document_hash(CATALOG_SCHEMA.as_bytes())
    );
    assert_eq!(record.inputs.catalog_state, None);
    assert_eq!(
        record.inputs.query,
        document_hash(amount_by_region().as_bytes())
    );
    assert_eq!(
        record.inputs.security_fingerprint,
        Some(plan.cache_key.security_fingerprint.clone())
    );
}

#[test]
fn pln_6_3_a_catalog_state_is_hashed_as_passed_even_when_it_is_ignored() {
    let state = "not a catalog state";
    let handle = handle_with(MODEL, CATALOG_SCHEMA, Some(state), &LoadOptions::default());
    let compilation = run(&handle, &amount_by_region(), &analyst(), &audited(None));
    assert_eq!(
        record(&compilation).inputs.catalog_state,
        Some(document_hash(state.as_bytes()))
    );
}

#[test]
fn pln_6_2_the_outcome_of_a_plan_is_its_plan_hash() {
    let compilation = run(&handle(), &amount_by_region(), &analyst(), &audited(None));
    let plan = plan(compilation.result.clone());
    let record = record(&compilation);
    assert_eq!(record.subject.as_deref(), Some("analyst-1"));
    match &record.outcome {
        Outcome::Plan(outcome) => assert_eq!(outcome.plan_hash, plan.cache_key.plan_hash),
        Outcome::Error(outcome) => panic!("expected a plan hash, got {outcome:?}"),
    }
}

#[test]
fn pln_6_5_every_authorization_decision_is_recorded_in_order() {
    let compilation = run(&handle(), &amount_by_region(), &analyst(), &audited(None));
    assert_eq!(
        entries(record(&compilation)),
        [
            ("dataset:ORDERS", Access::Granted, "access:roles"),
            (
                "field:ORDERS.REGION",
                Access::Granted,
                "no_field_restriction"
            ),
            ("metric:TOTAL_AMOUNT", Access::Granted, "metric_components"),
        ]
    );
}

#[test]
fn pln_6_6_the_row_policies_applied_and_the_decisions_are_recorded() {
    let compilation = run(&handle(), &amount_by_region(), &analyst(), &audited(None));
    let record = record(&compilation);
    let plan = plan(compilation.result.clone());
    assert_eq!(record.row_policies, ["ORDERS.REGION_SCOPE"]);
    assert_eq!(record.decisions, plan.explanation);
    let unrestricted = run(&handle(), &amount_by_region(), &admin(), &audited(None));
    assert!(self::record(&unrestricted).row_policies.is_empty());
}

#[test]
fn pln_6_7_the_options_and_contract_versions_are_recorded() {
    let compilation = run(&handle(), &amount_by_region(), &analyst(), &audited(None));
    let record = serde_json::to_value(record(&compilation)).unwrap();
    assert_eq!(
        record["compile_options"],
        serde_json::json!({"dialect": "duckdb"})
    );
    assert_eq!(
        record["load_options"],
        serde_json::json!({
            "grant_authenticated_to_all": false,
            "ossie_format": "release",
            "namespace": "menura",
            "data_source": "warehouse:shop",
        })
    );
    assert_eq!(
        record["contract_versions"],
        serde_json::json!({
            "model": "0.1.1",
            "catalog_schema": "1.1.0",
            "query": "1.1.0",
            "security_context": "1.1.0",
        })
    );
    assert_eq!(record["contract"], "menura.audit_record");
}

#[test]
fn pln_6_7_model_name_is_recorded_only_when_it_was_passed() {
    let options = LoadOptions {
        model_name: Some("shop".to_owned()),
        ..LoadOptions::default()
    };
    let handle = handle_with(MODEL, CATALOG_SCHEMA, None, &options);
    let compilation = run(&handle, &amount_by_region(), &analyst(), &audited(None));
    assert_eq!(
        record(&compilation).load_options.model_name.as_deref(),
        Some("shop")
    );
}

#[test]
fn pln_6_8_the_actor_is_written_unchanged_and_changes_no_hash() {
    let with_actor = run(
        &handle(),
        &amount_by_region(),
        &analyst(),
        &audited(Some("agent:report-assistant")),
    );
    let without_actor = run(&handle(), &amount_by_region(), &analyst(), &audited(None));
    let (with, without) = (record(&with_actor), record(&without_actor));
    assert_eq!(with.actor.as_deref(), Some("agent:report-assistant"));
    assert_eq!(without.actor, None);
    assert_eq!(with.inputs, without.inputs);
    assert_eq!(with.outcome, without.outcome);
    let without_json = serde_json::to_value(without).unwrap();
    assert!(without_json.get("actor").is_none());
}

#[test]
fn pln_6_4_a_context_not_admitted_has_no_subject_and_no_fingerprint() {
    let compilation = run(&handle(), &amount_by_region(), "{", &audited(None));
    let document = error(compilation.result.clone());
    assert_eq!(document.errors[0].code, "ESEC-01");
    let record = record(&compilation);
    assert_eq!(record.subject, None);
    assert_eq!(record.inputs.security_fingerprint, None);
    assert_eq!(
        record.inputs.query,
        document_hash(amount_by_region().as_bytes())
    );
    assert!(record.authorization.is_empty());
    match &record.outcome {
        Outcome::Error(outcome) => assert_eq!(outcome.error_codes, ["ESEC-01"]),
        Outcome::Plan(outcome) => panic!("expected error codes, got {outcome:?}"),
    }
    assert!(!record.contract_versions.contains_key("security_context"));
}

#[test]
fn pln_6_5_a_denied_object_is_recorded_before_the_error() {
    let outsider = context("outsider-1", &[], "{}");
    let compilation = run(&handle(), &amount_by_region(), &outsider, &audited(None));
    assert_eq!(
        error(compilation.result.clone()).errors[0].code,
        "E_NAME_NOT_FOUND"
    );
    let record = record(&compilation);
    assert_eq!(record.subject.as_deref(), Some("outsider-1"));
    assert_eq!(
        entries(record),
        [("dataset:ORDERS", Access::Denied, "access:roles")]
    );
    assert!(record.row_policies.is_empty());
    match &record.outcome {
        Outcome::Error(outcome) => assert_eq!(outcome.error_codes, ["E_NAME_NOT_FOUND"]),
        Outcome::Plan(outcome) => panic!("expected error codes, got {outcome:?}"),
    }
}

#[test]
fn pln_6_6_the_decisions_of_an_error_are_those_recorded_before_it() {
    let compilation = run(&handle(), &amount_by_region(), "{", &audited(None));
    let record = record(&compilation);
    let steps: Vec<_> = record.decisions.iter().map(|entry| entry.step).collect();
    let full = run(&handle(), &amount_by_region(), &analyst(), &audited(None));
    let e0_entries = self::record(&full)
        .decisions
        .iter()
        .filter(|entry| entry.step == menura_contracts::types::common::Step::E0)
        .count();
    assert_eq!(steps.len(), e0_entries);
}

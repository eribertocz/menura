//! The public function `explain`: PLN-8, PLN-8.1, PLN-8.2 and PLN-8.3.

mod support;

use menura_contracts::types::compiled_plan::CompiledPlanContract;
use menura_core::{CompileOptions, CompileResult, CompiledPlanDocument, Handle, compile, explain};
use support::{amount_by_region, analyst, compile_default, error, handle, plan, query};

fn explain_with(
    handle: &Handle,
    query: &str,
    context: &str,
    options: &CompileOptions,
) -> CompileResult {
    explain(handle, query.as_bytes(), context.as_bytes(), options)
}

fn explain_default(handle: &Handle, query: &str, context: &str) -> CompileResult {
    explain_with(handle, query, context, &CompileOptions::default())
}

/// The compiled plan `compiled` as `explain` must return it (PLN-8.1).
fn explained(compiled: CompiledPlanDocument) -> CompiledPlanDocument {
    CompiledPlanDocument {
        contract: CompiledPlanContract::Explanation,
        sql: String::new(),
        parameters: Vec::new(),
        ..compiled
    }
}

#[test]
fn pln_8_explain_returns_a_document_for_every_input_compile_accepts() {
    let explained = plan(explain_default(&handle(), &amount_by_region(), &analyst()));
    assert_eq!(explained.contract, CompiledPlanContract::Explanation);
}

#[test]
fn pln_8_1_explain_returns_the_compiled_plan_without_sql_and_parameters() {
    let handle = handle();
    let compiled = plan(compile_default(&handle, &amount_by_region(), &analyst()));
    assert!(!compiled.sql.is_empty());
    assert!(
        !compiled.parameters.is_empty(),
        "the row policy binds the regions"
    );
    let document = plan(explain_default(&handle, &amount_by_region(), &analyst()));
    assert_eq!(document, explained(compiled));
}

#[test]
fn pln_8_1_the_cache_key_is_computed_from_the_real_plan() {
    let handle = handle();
    let compiled = plan(compile_default(&handle, &amount_by_region(), &analyst()));
    let document = plan(explain_default(&handle, &amount_by_region(), &analyst()));
    assert_eq!(document.cache_key, compiled.cache_key);
}

#[test]
fn pln_8_1_explain_keeps_the_compile_options_written_to_the_plan() {
    let handle = handle();
    let options = CompileOptions {
        max_rows: std::num::NonZeroU64::new(10),
        ..CompileOptions::default()
    };
    let compiled = compile(
        &handle,
        amount_by_region().as_bytes(),
        analyst().as_bytes(),
        &options,
    );
    let document = explain_with(&handle, &amount_by_region(), &analyst(), &options);
    assert_eq!(plan(document), explained(plan(compiled)));
}

#[test]
fn pln_8_2_explain_returns_the_identical_compile_error() {
    let handle = handle();
    let unknown = query(r#"{"dimensions": ["orders.nowhere"]}"#);
    let compiled = error(compile_default(&handle, &unknown, &analyst()));
    let explained = error(explain_default(&handle, &unknown, &analyst()));
    assert_eq!(explained, compiled);
}

#[test]
fn pln_8_2_a_security_context_error_is_identical() {
    let handle = handle();
    let compiled = error(compile_default(&handle, &amount_by_region(), "not json"));
    let explained = error(explain_default(&handle, &amount_by_region(), "not json"));
    assert_eq!(explained, compiled);
}

#[test]
fn pln_8_3_explain_ignores_the_audit_option() {
    let handle = handle();
    let audited = CompileOptions {
        audit: true,
        ..CompileOptions::default()
    };
    let with_audit = plan(explain_with(
        &handle,
        &amount_by_region(),
        &analyst(),
        &audited,
    ));
    let without_audit = plan(explain_default(&handle, &amount_by_region(), &analyst()));
    assert_eq!(with_audit, without_audit);
}

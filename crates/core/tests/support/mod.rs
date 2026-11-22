//! The `shop` model of the compile tests, its documents and helpers that read a compile result.

#![cfg(test)]
#![allow(
    dead_code,
    reason = "each test file uses a different part of the shared support module"
)]

use menura_core::{
    CompileErrorDocument, CompileOptions, CompileResult, CompiledPlanDocument, Handle, LoadOptions,
    compile, load,
};

/// The model: `orders` with the `region_scope` row policy for `analyst`, and `customers`.
pub const MODEL: &str = include_str!("../fixtures/shop/model.yaml");
/// The catalog schema of the model, whose `document_id.origin` is `warehouse:shop`.
pub const CATALOG_SCHEMA: &str = include_str!("../fixtures/shop/catalog-schema.json");

/// The handle of the model, without catalog state.
pub fn handle() -> Handle {
    handle_with(MODEL, CATALOG_SCHEMA, None, &LoadOptions::default())
}

/// The handle of `model` against `catalog_schema`, with `state` when given.
pub fn handle_with(
    model: &str,
    catalog_schema: &str,
    state: Option<&str>,
    options: &LoadOptions,
) -> Handle {
    load(
        model.as_bytes(),
        catalog_schema.as_bytes(),
        state.map(str::as_bytes),
        options,
    )
    .unwrap()
}

/// A query document of the wide-table form whose `query` member is `query`.
pub fn query(query: &str) -> String {
    format!(
        r#"{{"contract": "menura.query", "contract_version": "1.1.0", "time_zone": "UTC",
            "query": {query}}}"#
    )
}

/// The query of `total_amount` by `orders.region`.
pub fn amount_by_region() -> String {
    query(r#"{"dimensions": ["orders.region"], "metrics": ["total_amount"]}"#)
}

/// A security context with `subject`, `roles` and the `attributes` object text.
pub fn context(subject: &str, roles: &[&str], attributes: &str) -> String {
    let roles: Vec<String> = roles.iter().map(|role| format!("\"{role}\"")).collect();
    format!(
        r#"{{"contract": "menura.security_context", "contract_version": "1.1.0",
            "subject": "{subject}", "roles": [{}], "attributes": {attributes}}}"#,
        roles.join(", ")
    )
}

/// An analyst who sees the regions `NORTH` and `SOUTH`.
pub fn analyst() -> String {
    context(
        "analyst-1",
        &["analyst"],
        r#"{"regions": ["NORTH", "SOUTH"]}"#,
    )
}

/// Compiles `query` for `context` with the default options.
pub fn compile_default(handle: &Handle, query: &str, context: &str) -> CompileResult {
    compile(
        handle,
        query.as_bytes(),
        context.as_bytes(),
        &CompileOptions::default(),
    )
    .result
}

/// The compiled plan of `result`; fails the test otherwise.
pub fn plan(result: CompileResult) -> CompiledPlanDocument {
    match result {
        CompileResult::Plan(plan) => *plan,
        other @ CompileResult::Error(_) => panic!("expected a compiled plan, got {other:?}"),
    }
}

/// The compile error of `result`; fails the test otherwise.
pub fn error(result: CompileResult) -> CompileErrorDocument {
    match result {
        CompileResult::Error(error) => error,
        other @ CompileResult::Plan(_) => panic!("expected a compile error, got {other:?}"),
    }
}

/// The code of the first error of `result`.
pub fn first_code(result: CompileResult) -> String {
    error(result).errors[0].code.clone()
}

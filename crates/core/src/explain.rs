//! The public function `explain` (architecture section 3.5): the compiled plan without its SQL
//! and parameters (PLN-8).

use menura_contracts::types::compiled_plan::CompiledPlanContract;

use crate::compile::{CompileResult, compile};
use crate::compile_options::CompileOptions;
use crate::handle::Handle;

/// Explains the query document `query` for the user of the security context document
/// `security_context`, over the model of `handle`, with `options` (PLN-8).
///
/// The result is exactly the document [`compile`] returns for the same inputs, with `contract`
/// `menura.explanation`, `sql` `""` and `parameters` `[]` (PLN-8.1). Its `cache_key` is
/// computed from the real plan, so it equals the one `compile` returns. `explain` ignores the
/// `audit` option and never emits an audit record (PLN-8.3).
///
/// # Errors
///
/// [`CompileResult::Error`], the identical compile error `compile` returns for the same inputs
/// (PLN-8.2).
///
/// # Example
///
/// ```
/// use menura_core::{CompileOptions, CompileResult, LoadOptions, explain, load};
/// # let model = include_bytes!("../tests/fixtures/shop/model.yaml");
/// # let catalog_schema = include_bytes!("../tests/fixtures/shop/catalog-schema.json");
///
/// let handle = load(model, catalog_schema, None, &LoadOptions::default()).unwrap();
/// let query = br#"{"contract": "menura.query", "contract_version": "1.1.0",
///     "time_zone": "UTC", "query": {"dimensions": ["orders.region"]}}"#;
/// let context = br#"{"contract": "menura.security_context", "contract_version": "1.1.0",
///     "subject": "user-1", "roles": ["admin"], "attributes": {}}"#;
/// let CompileResult::Plan(explanation) = explain(&handle, query, context, &CompileOptions::default())
/// else {
///     panic!("the query is explained");
/// };
/// assert!(explanation.sql.is_empty());
/// assert!(explanation.parameters.is_empty());
/// ```
#[must_use]
pub fn explain(
    handle: &Handle,
    query: &[u8],
    security_context: &[u8],
    options: &CompileOptions,
) -> CompileResult {
    let without_audit = CompileOptions {
        audit: false,
        ..*options
    };
    match compile(handle, query, security_context, &without_audit) {
        CompileResult::Plan(mut plan) => {
            plan.contract = CompiledPlanContract::Explanation;
            plan.sql = String::new();
            plan.parameters = Vec::new();
            CompileResult::Plan(plan)
        }
        error @ CompileResult::Error(_) => error,
    }
}

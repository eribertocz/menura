//! The public function `compile` (architecture section 3.5): the stages in the order of E2-5,
//! and the compiled plan or compile error it returns (PLN-1, PLN-5).

use menura_contracts::ContractKind;
use menura_contracts::types::compiled_plan::{
    CompileErrorContract, CompileErrorDocument, CompiledPlanDocument, Diagnostic,
};
use menura_e1_parse::{ParseWarning, parse_query};
use menura_e2_bind::bind;
use menura_e3_paths::resolve_paths;
use menura_e4_plan::build_plan;
use menura_e5_optimize::optimize;
use menura_generator_duckdb::generate_sql;
use menura_security::{AdmittedSecurityContext, admit_security_context};

use crate::compile_options::CompileOptions;
use crate::explanation::assemble;
use crate::handle::Handle;
use crate::plan_document::{PlanOutputs, plan_document};
use crate::stop::{
    Stop, audit_not_yet_supported, bind_stop, context_not_utf8, path_stop, plan_stop,
    query_not_utf8,
};

/// What `compile` returns: a compiled plan or a compile error (PLN-1, PLN-5).
#[derive(Debug, Clone, PartialEq)]
pub enum CompileResult {
    /// The compiled plan (PLN-1).
    Plan(Box<CompiledPlanDocument>),
    /// The compile error (PLN-5).
    Error(CompileErrorDocument),
}

/// A defect of the compiler: the stage that found it and what it found. `compile` reports it
/// as `E_INTERNAL` (PLN-5.1a).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InternalError {
    /// The stage that found the defect, such as `E3`.
    pub(crate) stage: &'static str,
    /// What the stage found.
    pub(crate) message: String,
}

impl InternalError {
    /// The defect `message` found by `stage`.
    pub(crate) fn new(stage: &'static str, message: impl Into<String>) -> Self {
        Self {
            stage,
            message: message.into(),
        }
    }
}

/// Compiles the query document `query` for the user of the security context document
/// `security_context`, over the model of `handle`, with `options` (architecture section 3.5).
///
/// The stages run in the order of E2-5: the security context is admitted against the model
/// (row 0a), then E1, E2, E3, E4, E5 and E6. E5 reads the handle's catalog state, if any
/// (E5-2.1), and E6 generates the optimized plan. The first error stops compilation.
///
/// The warnings, of the plan or of the error, are the handle's load warnings, then the
/// security context's `WQRY-01` (SEC-3.9), then the query's (CON-0.5).
///
/// # Errors
///
/// [`CompileResult::Error`], the compile error document (PLN-5) with the first error and the
/// warnings gathered before it: `E_NOT_YET_SUPPORTED` for `audit: true` (VS-5); `ESEC-01` or
/// `ESEC-03` for the security context (SEC-3.8, SEC-3.1); then the errors of E1, E2, E3 and E4, in
/// the order of E2-5; `E_INTERNAL`, without location, for a defect of the compiler
/// (PLN-5.1a).
///
/// # Example
///
/// ```
/// use menura_core::{CompileOptions, CompileResult, LoadOptions, compile, load};
/// # let model = include_bytes!("../tests/fixtures/shop/model.yaml");
/// # let catalog_schema = include_bytes!("../tests/fixtures/shop/catalog-schema.json");
///
/// let handle = load(model, catalog_schema, None, &LoadOptions::default()).unwrap();
/// let query = br#"{"contract": "menura.query", "contract_version": "1.1.0",
///     "time_zone": "UTC", "query": {"dimensions": ["orders.region"]}}"#;
/// let context = br#"{"contract": "menura.security_context", "contract_version": "1.1.0",
///     "subject": "user-1", "roles": ["admin"], "attributes": {}}"#;
/// let CompileResult::Plan(plan) = compile(&handle, query, context, &CompileOptions::default())
/// else {
///     panic!("the query compiles");
/// };
/// assert!(plan.sql.starts_with("WITH"));
/// ```
#[must_use]
pub fn compile(
    handle: &Handle,
    query: &[u8],
    security_context: &[u8],
    options: &CompileOptions,
) -> CompileResult {
    let mut warnings: Vec<Diagnostic> = handle.warnings().cloned().collect();
    match compile_stages(handle, query, security_context, options, &mut warnings) {
        Ok(plan) => CompileResult::Plan(Box::new(plan)),
        Err(stop) => CompileResult::Error(CompileErrorDocument {
            contract: CompileErrorContract::CompileError,
            contract_version: ContractKind::CompileError.latest_version(),
            errors: vec![stop.into_diagnostic()],
            warnings,
        }),
    }
}

/// Runs the stages, adding the warnings of each to `warnings`, and builds the plan.
fn compile_stages(
    handle: &Handle,
    query: &[u8],
    security_context: &[u8],
    options: &CompileOptions,
    warnings: &mut Vec<Diagnostic>,
) -> Result<CompiledPlanDocument, Stop> {
    if options.audit {
        return Err(audit_not_yet_supported().into());
    }
    let context = admit_context(handle, security_context)?;
    warnings.extend(context.former_name_warning());
    let parsed = match parse_query(utf8(query).ok_or_else(query_not_utf8)?) {
        Ok(parsed) => parsed,
        Err(error) => {
            warnings.extend(error.warnings.iter().map(ParseWarning::diagnostic));
            return Err(error.diagnostic().into());
        }
    };
    warnings.extend(parsed.warnings.iter().map(ParseWarning::diagnostic));
    let model = handle.model();
    let bound = bind(&parsed.tree, model, &context).map_err(bind_stop)?;
    let mut entries = handle.explanation().to_vec();
    entries.extend(menura_e2_bind::explanation(&parsed.tree, &bound, &context));
    let branches = resolve_paths(&bound, model).map_err(|error| path_stop(&error))?;
    entries.extend(menura_e3_paths::explanation(&bound));
    let plan = build_plan(&branches, &bound, &context, model, handle.catalog_schema())
        .map_err(|error| plan_stop(&error))?;
    entries.extend(
        menura_e4_plan::explanation(&plan, &branches, &bound, &context, model)
            .map_err(|error| plan_stop(&error))?,
    );
    let optimized = optimize(
        &plan,
        &bound,
        model,
        handle.catalog_schema(),
        handle.catalog_state(),
        &context,
    );
    entries.extend_from_slice(optimized.explanation());
    let plan = optimized.into_plan();
    let generated = generate_sql(&plan, handle.catalog_schema(), model, &bound, &context)
        .map_err(|error| InternalError::new("E6", format!("{error:?}")))?;
    Ok(plan_document(
        handle,
        &plan,
        &bound,
        generated,
        &context,
        options,
        PlanOutputs {
            explanation: assemble(&entries),
            warnings: std::mem::take(warnings),
        },
    )?)
}

/// Admits the security context `bytes` against the model's declarations (E2-5 row 0a).
fn admit_context(handle: &Handle, bytes: &[u8]) -> Result<AdmittedSecurityContext, Stop> {
    let text = utf8(bytes).ok_or_else(context_not_utf8)?;
    let declarations = handle.model().parts().security_context.as_ref();
    admit_security_context(text, declarations).map_err(|error| error.diagnostic().into())
}

fn utf8(bytes: &[u8]) -> Option<&str> {
    std::str::from_utf8(bytes).ok()
}

//! The public function `compile` (architecture section 3.5): the stages in the order of E2-5,
//! and the compiled plan or compile error it returns (PLN-1, PLN-5).

use std::fmt;

use menura_contracts::ContractKind;
use menura_contracts::types::compiled_plan::{
    CompileErrorContract, CompileErrorDocument, CompiledPlanDocument, Diagnostic,
};
use menura_e1_parse::{ParseWarning, parse_query};
use menura_e2_bind::bind;
use menura_e3_paths::resolve_paths;
use menura_e4_plan::build_plan;
use menura_generator_duckdb::generate_sql;
use menura_security::{AdmittedSecurityContext, admit_security_context};

use crate::compile_options::CompileOptions;
use crate::handle::Handle;
use crate::plan_document::plan_document;
use crate::stop::{
    Stop, audit_not_yet_supported, bind_stop, context_not_utf8, plan_stop, query_not_utf8,
};

/// What `compile` returns: a compiled plan or a compile error (PLN-1, PLN-5), or an internal
/// error of the compiler.
#[derive(Debug, Clone, PartialEq)]
pub enum CompileResult {
    /// The compiled plan (PLN-1).
    Plan(Box<CompiledPlanDocument>),
    /// The compile error (PLN-5).
    Error(CompileErrorDocument),
    /// A stage received an input that the stage before it never produces: a defect of the
    /// compiler, not a query error, so it has no specification code.
    Internal(InternalError),
}

/// A defect of the compiler: the stage that found it and what it found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalError {
    /// The stage that found the defect, such as `E3`.
    pub stage: &'static str,
    /// What the stage found.
    pub message: String,
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

impl fmt::Display for InternalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "internal error in {}: {}",
            self.stage, self.message
        )
    }
}

impl std::error::Error for InternalError {}

/// Compiles the query document `query` for the user of the security context document
/// `security_context`, over the model of `handle`, with `options` (architecture section 3.5).
///
/// The stages run in the order of E2-5: the security context is admitted against the model
/// (row 0a), then E1, E2, E3, E4 and E6; E5 is not part of 0.3.0, so the plan of E4 goes to
/// E6 unchanged. The first error stops compilation.
///
/// The warnings, of the plan or of the error, are the handle's load warnings, then the
/// security context's `WQRY-01` (SEC-3.9), then the query's (CON-0.5).
///
/// # Errors
///
/// [`CompileResult::Error`], the compile error document (PLN-5) with the first error and the
/// warnings gathered before it: `E_NOT_YET_SUPPORTED` for `audit: true` (VS-5); `ESEC-01` or
/// `ESEC-03` for the security context (SEC-3.8, SEC-3.1); then the errors of E1, E2 and E4, in
/// the order of E2-5. [`CompileResult::Internal`] for a defect of the compiler.
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
        Err(Stop::Error(error)) => CompileResult::Error(CompileErrorDocument {
            contract: CompileErrorContract::CompileError,
            contract_version: ContractKind::CompileError.latest_version(),
            errors: vec![error],
            warnings,
        }),
        Err(Stop::Internal(error)) => CompileResult::Internal(error),
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
    let parsed =
        parse_query(utf8(query).ok_or_else(query_not_utf8)?).map_err(|error| error.diagnostic())?;
    warnings.extend(parsed.warnings.iter().map(ParseWarning::diagnostic));
    let model = handle.model();
    let bound = bind(&parsed.tree, model, &context).map_err(bind_stop)?;
    let branches = resolve_paths(&bound, model)
        .map_err(|violation| InternalError::new("E3", violation.to_string()))?;
    let plan = build_plan(&branches, &bound, &context, model).map_err(|error| plan_stop(&error))?;
    let generated = generate_sql(&plan, handle.catalog_schema(), model, &bound, &context)
        .map_err(|error| InternalError::new("E6", format!("{error:?}")))?;
    Ok(plan_document(
        handle,
        &plan,
        generated,
        &context,
        options,
        std::mem::take(warnings),
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

//! The public function `compile` (architecture section 3.5): the stages in the order of E2-5,
//! and the compiled plan or compile error it returns (PLN-1, PLN-5), with the audit record when
//! it is asked for (PLN-6).

use menura_contracts::ContractKind;
use menura_contracts::types::audit_record::AuditRecordDocument;
use menura_contracts::types::compiled_plan::{
    CompileErrorContract, CompileErrorDocument, CompiledPlanDocument, Diagnostic,
};
use menura_e1_parse::{ParseWarning, parse_query};
use menura_e2_bind::{authorization_decisions, bind};
use menura_e3_paths::{PathCode, PathError, resolve_paths};
use menura_e4_plan::{
    ContextBranches, ContextQuery, PlanInputs, build_plan_with_contexts, context_queries,
};
use menura_e5_optimize::optimize;
use menura_model::explanation::ExplanationEntry;
use menura_model::internal_model::InternalModel;
use menura_representations::bound_query::{AuthorizationDecision, BoundQuery};
use menura_representations::logical_plan::PlanNode;
use menura_representations::paths::Branches;
use menura_representations::query_tree::QueryTree;
use menura_security::{AdmittedSecurityContext, admit_security_context};

use crate::audit::{AuditFacts, audit_record, error_outcome, plan_outcome};
use crate::compile_options::CompileOptions;
use crate::dialect_sql::{Sources, query_sql};
use crate::explanation::assemble;
use crate::handle::Handle;
use crate::lineage::{LineageInputs, row_policies};
use crate::passed_document::PassedDocument;
use crate::plan_document::{PlanOutputs, plan_document};
use crate::stop::{Stop, bind_stop, context_not_utf8, path_stop, plan_stop, query_not_utf8};

/// What `compile` returns: a compiled plan or a compile error (PLN-1, PLN-5).
#[derive(Debug, Clone, PartialEq)]
pub enum CompileResult {
    /// The compiled plan (PLN-1).
    Plan(Box<CompiledPlanDocument>),
    /// The compile error (PLN-5).
    Error(CompileErrorDocument),
}

/// What one call of `compile` returns: the compiled plan or the compile error, and, with
/// `audit: true`, the audit record of the same compilation (architecture section 3.5, VS-1e).
#[derive(Debug, Clone, PartialEq)]
pub struct Compilation {
    /// The compiled plan or the compile error; the same with `audit: true` as with
    /// `audit: false` (VS-1e).
    pub result: CompileResult,
    /// The audit record (PLN-6) with `audit: true`, `None` with `audit: false`.
    pub audit: Option<Box<AuditRecordDocument>>,
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

/// What the stages recorded for the audit record before they stopped (PLN-6.5, PLN-6.6).
struct Trace {
    /// Every authorization decision of E2, in order (E2-4.7).
    authorization: Vec<AuthorizationDecision>,
    /// The explanation entries of every stage that completed, E0's first.
    entries: Vec<ExplanationEntry>,
    /// Every row policy applied to the final plan.
    row_policies: Vec<String>,
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
/// With `audit: true`, the result also carries the audit record of the compilation (PLN-6),
/// with the plan hash or the error codes as its outcome; the plan or the error is the same as
/// with `audit: false` (VS-1e). The `actor` option is written only in the audit record
/// (PLN-6.8).
///
/// # Errors
///
/// [`CompileResult::Error`], the compile error document (PLN-5) with the first error and the
/// warnings gathered before it: `ESEC-01` or `ESEC-03` for the security context (SEC-3.8,
/// SEC-3.1); then the errors of E1, E2, E3 and E4, in the order of E2-5; `E6-01` when the
/// `clickhouse` dialect cannot express a value of the plan (E6-7.12); `E_INTERNAL`, without
/// location, for a defect of the compiler (PLN-5.1a).
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
/// let options = CompileOptions { audit: true, ..CompileOptions::default() };
/// let compilation = compile(&handle, query, context, &options);
/// let CompileResult::Plan(plan) = compilation.result else {
///     panic!("the query compiles");
/// };
/// assert!(plan.sql.starts_with("WITH"));
/// assert_eq!(compilation.audit.unwrap().subject.as_deref(), Some("user-1"));
/// ```
#[must_use]
pub fn compile(
    handle: &Handle,
    query: &[u8],
    security_context: &[u8],
    options: &CompileOptions,
) -> Compilation {
    let mut warnings: Vec<Diagnostic> = handle.warnings().cloned().collect();
    let mut trace = Trace {
        authorization: Vec::new(),
        entries: handle.explanation().to_vec(),
        row_policies: Vec::new(),
    };
    let context = admit_context(handle, security_context);
    let outcome = match &context {
        Ok(context) => {
            warnings.extend(context.former_name_warning());
            compile_stages(handle, query, context, options, &mut warnings, &mut trace)
        }
        Err(stop) => Err(stop.clone()),
    };
    let result = match outcome {
        Ok(plan) => CompileResult::Plan(Box::new(plan)),
        Err(stop) => CompileResult::Error(error_document(stop, warnings)),
    };
    if !options.audit {
        return Compilation {
            result,
            audit: None,
        };
    }
    let documents = PassedInputs {
        query: PassedDocument::of(query),
        security_context: PassedDocument::of(security_context),
    };
    with_audit(
        handle,
        options,
        result,
        context.as_ref().ok(),
        &documents,
        trace,
    )
}

/// The query and the security context documents as passed (PLN-6.3).
struct PassedInputs {
    query: PassedDocument,
    security_context: PassedDocument,
}

/// The compilation of `result` with its audit record (PLN-6). When the record cannot be
/// written, the result is `E_INTERNAL` (PLN-5.1a), without record.
fn with_audit(
    handle: &Handle,
    options: &CompileOptions,
    result: CompileResult,
    context: Option<&AdmittedSecurityContext>,
    documents: &PassedInputs,
    trace: Trace,
) -> Compilation {
    let outcome = match &result {
        CompileResult::Plan(plan) => plan_outcome(plan.cache_key.plan_hash.clone()),
        CompileResult::Error(error) => error_outcome(
            error
                .errors
                .iter()
                .map(|diagnostic| diagnostic.code.clone())
                .collect(),
        ),
    };
    let facts = AuditFacts {
        query: &documents.query,
        security_context: &documents.security_context,
        context,
        outcome,
        authorization: &trace.authorization,
        row_policies: trace.row_policies,
        decisions: assemble(&trace.entries),
    };
    match audit_record(handle, options, facts) {
        Ok(record) => Compilation {
            result,
            audit: Some(Box::new(record)),
        },
        Err(error) => Compilation {
            result: CompileResult::Error(error_document(
                error.into(),
                handle.warnings().cloned().collect(),
            )),
            audit: None,
        },
    }
}

/// The compile error document of `stop`, with `warnings`.
fn error_document(stop: Stop, warnings: Vec<Diagnostic>) -> CompileErrorDocument {
    CompileErrorDocument {
        contract: CompileErrorContract::CompileError,
        contract_version: ContractKind::CompileError.latest_version(),
        errors: vec![stop.into_diagnostic()],
        warnings,
    }
}

/// Runs the stages after the security context is admitted, adding the warnings of each to
/// `warnings` and what the audit record needs to `trace`, and builds the plan.
fn compile_stages(
    handle: &Handle,
    query: &[u8],
    context: &AdmittedSecurityContext,
    options: &CompileOptions,
    warnings: &mut Vec<Diagnostic>,
    trace: &mut Trace,
) -> Result<CompiledPlanDocument, Stop> {
    let parsed = match parse_query(utf8(query).ok_or_else(query_not_utf8)?) {
        Ok(parsed) => parsed,
        Err(error) => {
            warnings.extend(error.warnings.iter().map(ParseWarning::diagnostic));
            return Err(error.diagnostic().into());
        }
    };
    warnings.extend(parsed.warnings.iter().map(ParseWarning::diagnostic));
    let model = handle.model();
    let bound = bind_tree(&parsed.tree, model, context, options, trace)?;
    let plan = logical_plan(handle, &bound, context, trace)?;
    let optimized = optimize(
        &plan,
        &bound,
        model,
        handle.catalog_schema(),
        handle.catalog_state(),
        context,
    );
    trace.entries.extend_from_slice(optimized.explanation());
    let plan = optimized.into_plan();
    let sources = Sources {
        catalog: handle.catalog_schema(),
        model,
    };
    let generated = query_sql(options.dialect, &plan, sources, &bound, context)?;
    let lineage_inputs = LineageInputs {
        model,
        catalog: handle.catalog_schema(),
        context,
    };
    if options.audit {
        trace.row_policies = row_policies(&plan, lineage_inputs)?;
    }
    Ok(plan_document(
        handle,
        &plan,
        &bound,
        generated,
        context,
        options,
        PlanOutputs {
            explanation: assemble(&trace.entries),
            warnings: std::mem::take(warnings),
        },
    )?)
}

/// The logical plan of `bound` (E3, E4): the branches of the query and of each of its context
/// queries (E4-8.2), then the plan built from them, adding the explanation entries of E3 and
/// E4 to `trace`.
fn logical_plan(
    handle: &Handle,
    bound: &BoundQuery,
    context: &AdmittedSecurityContext,
    trace: &mut Trace,
) -> Result<PlanNode, Stop> {
    let model = handle.model();
    let branches = resolve_paths(bound, model).map_err(|error| path_stop(&error))?;
    trace.entries.extend(menura_e3_paths::explanation(bound));
    let mut context_queries = context_queries(bound, model).map_err(|error| plan_stop(&error))?;
    let context_branches = context_queries
        .iter_mut()
        .map(|context| context_paths(context, model))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| path_stop(&error))?;
    let contexts: Vec<ContextBranches<'_>> = context_queries
        .iter()
        .zip(&context_branches)
        .map(|(context, branches)| ContextBranches { context, branches })
        .collect();
    let inputs = PlanInputs {
        branches: &branches,
        query: bound,
        contexts: &contexts,
    };
    let plan = build_plan_with_contexts(&inputs, context, model, handle.catalog_schema())
        .map_err(|error| plan_stop(&error))?;
    trace.entries.extend(
        menura_e4_plan::explanation_with_contexts(&plan, &inputs, context, model)
            .map_err(|error| plan_stop(&error))?,
    );
    Ok(plan)
}

/// The branches of the context query `context` (E3). It keeps only the query's `paths`
/// entries that its own branches use (E4-8.2): each entry E3 reports unused (`E3-01`) is taken
/// out, and the branches are resolved again.
fn context_paths(context: &mut ContextQuery, model: &InternalModel) -> Result<Branches, PathError> {
    loop {
        match resolve_paths(context.query(), model) {
            Err(PathError::Diagnostic(diagnostic))
                if diagnostic.code == PathCode::UnusedPathEntry
                    && context.without_path(&diagnostic.location) => {}
            resolved => return resolved,
        }
    }
}

/// Binds `tree` (E2), recording its authorization decisions, also when it fails, and its
/// explanation entries in `trace`.
fn bind_tree(
    tree: &QueryTree,
    model: &InternalModel,
    context: &AdmittedSecurityContext,
    options: &CompileOptions,
    trace: &mut Trace,
) -> Result<BoundQuery, Stop> {
    let bound = match bind(tree, model, context) {
        Ok(bound) => bound,
        Err(error) => {
            if options.audit {
                trace.authorization = authorization_decisions(tree, model, context);
            }
            return Err(bind_stop(error));
        }
    };
    trace.authorization = bound.authorization().to_vec();
    trace
        .entries
        .extend(menura_e2_bind::explanation(tree, &bound, context));
    Ok(bound)
}

/// Admits the security context `bytes` against the model's declarations (E2-5 row 0a).
pub(crate) fn admit_context(
    handle: &Handle,
    bytes: &[u8],
) -> Result<AdmittedSecurityContext, Stop> {
    let text = utf8(bytes).ok_or_else(context_not_utf8)?;
    let declarations = handle.model().parts().security_context.as_ref();
    admit_security_context(text, declarations).map_err(|error| error.diagnostic().into())
}

fn utf8(bytes: &[u8]) -> Option<&str> {
    std::str::from_utf8(bytes).ok()
}

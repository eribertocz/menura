//! What a wrapper call must give: the documents `core` gives for the same inputs, composed as
//! each command composes them (ARC-5.4 to ARC-5.9).

use menura_core::{
    Argument, CommandName, CompileErrorDocument, CompileOptions, CompileResult, DescribeResult,
    Dialect, Handle, LoadOptions, OutputError, UsageError, audit_record_json, compile,
    dependencies, dependencies_json, describe, description_json, diff, diff_json, document_json,
    explain, integrity_checks, integrity_checks_json, load, migrate, migration_report_json,
    parse_compile_options, parse_load_options, plan_json, validate,
};

use super::call::WrapperCall;
use super::error::ParityError;

/// The output of a call that runs: the CLI writes `document` and a line feed and exits `1`
/// exactly when `is_error` (ARC-5.4); MCP and Python return the same (ARC-6.5, ARC-6.8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedOutput {
    /// The output document, without a final line feed.
    pub document: String,
    /// Whether the document is a compile error or a report with errors.
    pub is_error: bool,
    /// The audit record of `compile` with `audit: true`, when `compile` ran (ARC-5.7).
    pub audit_record: Option<String>,
}

/// What a call must give.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expected {
    /// The call runs and gives this output.
    Output(ExpectedOutput),
    /// The call is refused with this usage error (ARC-5.5, ARC-6.4, ARC-6.7).
    Usage(UsageError),
}

/// The result `core` gives for `call`.
///
/// # Errors
///
/// Returns [`ParityError::Output`] when `core` cannot write a document of the call.
pub fn expected_result(call: &WrapperCall) -> Result<Expected, ParityError> {
    let computed = match compute(call) {
        Ok(computed) => computed,
        Err(Refusal::Usage(error)) => return Ok(Expected::Usage(error)),
        Err(Refusal::Output(error)) => {
            return Err(ParityError::Output {
                case: call.case.clone(),
                reason: error.to_string(),
            });
        }
    };
    Ok(Expected::Output(computed))
}

/// Why a call gives no output.
enum Refusal {
    Usage(UsageError),
    Output(OutputError),
}

impl From<UsageError> for Refusal {
    fn from(error: UsageError) -> Self {
        Self::Usage(error)
    }
}

impl From<OutputError> for Refusal {
    fn from(error: OutputError) -> Self {
        Self::Output(error)
    }
}

/// The documents of a call that is given every argument it requires.
fn compute(call: &WrapperCall) -> Result<ExpectedOutput, Refusal> {
    let options = load_options(call)?;
    let document = |argument| call.document(argument).unwrap_or_default();
    match call.command {
        CommandName::Validate => {
            let report = validate(
                document(Argument::Model),
                document(Argument::CatalogSchema),
                &options,
            );
            Ok(output(document_json(&report)?, !report.errors.is_empty()))
        }
        CommandName::Migrate => match migrate(document(Argument::Model), &options) {
            Ok(report) => Ok(output(migration_report_json(&report)?, false)),
            Err(error) => error_output(&error),
        },
        CommandName::Dependencies => model_document(call, &options, |handle| {
            dependencies(handle).map(|document| dependencies_json(&document))
        }),
        CommandName::IntegrityChecks => model_document(call, &options, |handle| {
            integrity_checks(handle, Dialect::Duckdb)
                .map(|document| integrity_checks_json(&document))
        }),
        CommandName::Describe => {
            let handle = match load_handle(call, &options) {
                Ok(handle) => handle,
                Err(error) => return error_output(&error),
            };
            match describe(&handle, document(Argument::SecurityContext)) {
                DescribeResult::Description(found) => Ok(output(description_json(&found)?, false)),
                DescribeResult::Error(error) => error_output(&error),
            }
        }
        CommandName::Compile | CommandName::Explain => compile_output(call, &options),
        CommandName::Diff => diff_output(call, &options),
    }
}

/// The load options of `call`, read as the CLI reads its file (ARC-5.5).
fn load_options(call: &WrapperCall) -> Result<LoadOptions, UsageError> {
    match call.options(Argument::LoadOptions) {
        Some(options) => parse_load_options(options.to_string().as_bytes()),
        None => Ok(LoadOptions::default()),
    }
}

/// The compile options of `call`, read as the CLI reads its file (ARC-5.7).
fn compile_options(call: &WrapperCall) -> Result<CompileOptions, UsageError> {
    match call.options(Argument::CompileOptions) {
        Some(options) => parse_compile_options(options.to_string().as_bytes()),
        None => Ok(CompileOptions::default()),
    }
}

/// Loads the model of `call` without a catalog state, as `dependencies` and
/// `integrity-checks` do, and writes what `write` gives for it (ARC-5.6, ARC-5.8).
fn model_document(
    call: &WrapperCall,
    options: &LoadOptions,
    write: impl Fn(&Handle) -> Result<Result<String, OutputError>, CompileErrorDocument>,
) -> Result<ExpectedOutput, Refusal> {
    let document = |argument| call.document(argument).unwrap_or_default();
    let loaded = load(
        document(Argument::Model),
        document(Argument::CatalogSchema),
        None,
        options,
    );
    match loaded.and_then(|handle| write(&handle)) {
        Ok(text) => Ok(output(text?, false)),
        Err(error) => error_output(&error),
    }
}

/// Loads the handle of `call`, with its catalog state when given (ARC-5.7, ARC-5.8).
fn load_handle(call: &WrapperCall, options: &LoadOptions) -> Result<Handle, CompileErrorDocument> {
    load(
        call.document(Argument::Model).unwrap_or_default(),
        call.document(Argument::CatalogSchema).unwrap_or_default(),
        call.document(Argument::CatalogState),
        options,
    )
}

/// The output of `compile` or `explain`; `compile` also gives its audit record (ARC-5.7).
fn compile_output(call: &WrapperCall, options: &LoadOptions) -> Result<ExpectedOutput, Refusal> {
    let compile_options = compile_options(call)?;
    let handle = match load_handle(call, options) {
        Ok(handle) => handle,
        Err(error) => return error_output(&error),
    };
    let query = call.document(Argument::Query).unwrap_or_default();
    let context = call.document(Argument::SecurityContext).unwrap_or_default();
    if call.command == CommandName::Explain {
        return result_output(explain(&handle, query, context, &compile_options));
    }
    let compilation = compile(&handle, query, context, &compile_options);
    let audit_record = compilation
        .audit
        .as_ref()
        .map(|record| audit_record_json(record))
        .transpose()?;
    let mut result = result_output(compilation.result)?;
    result.audit_record = audit_record;
    Ok(result)
}

/// The output of `diff`: both models loaded with the same options, old first (ARC-5.6).
fn diff_output(call: &WrapperCall, options: &LoadOptions) -> Result<ExpectedOutput, Refusal> {
    let document = |argument| call.document(argument).unwrap_or_default();
    let old = load(
        document(Argument::OldModel),
        document(Argument::OldCatalogSchema),
        None,
        options,
    );
    let compared = old.and_then(|old| {
        let new = load(
            document(Argument::NewModel),
            document(Argument::NewCatalogSchema),
            None,
            options,
        )?;
        dependencies(&old).and_then(|old| dependencies(&new).and_then(|new| diff(&old, &new)))
    });
    match compared {
        Ok(found) => Ok(output(diff_json(&found)?, false)),
        Err(error) => error_output(&error),
    }
}

/// The output of a compile result: the plan or explanation, or the compile error.
fn result_output(result: CompileResult) -> Result<ExpectedOutput, Refusal> {
    match result {
        CompileResult::Plan(plan) => Ok(output(plan_json(&plan)?, false)),
        CompileResult::Error(error) => error_output(&error),
    }
}

/// The output of a compile error (ARC-5.4: exit `1`).
fn error_output(error: &CompileErrorDocument) -> Result<ExpectedOutput, Refusal> {
    Ok(output(document_json(error)?, true))
}

/// An output without an audit record.
fn output(document: String, is_error: bool) -> ExpectedOutput {
    ExpectedOutput {
        document,
        is_error,
        audit_record: None,
    }
}

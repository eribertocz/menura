//! Runs a tool call: the same `core` functions, in the same order, as the CLI command, with the
//! documents of the members instead of files (ARC-6.1, ARC-6.8).

use menura_core::{
    Argument, CommandName, CompileErrorDocument, CompileOptions, CompileResult, DescribeResult,
    Dialect, Handle, LoadOptions, OutputError, UsageError, audit_record_json, compile,
    dependencies, dependencies_json, describe, description_json, diff, diff_json, document_json,
    explain, integrity_checks, integrity_checks_json, load, migrate, migration_report_json,
    parse_compile_options, parse_dialect, parse_load_options, plan_json, validate,
};

use crate::arguments::ToolCall;

/// The result of a tool: its text content items and whether the CLI would exit `1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolOutput {
    /// The output document, then the audit record when there is one (ARC-6.8).
    pub(crate) texts: Vec<String>,
    /// True exactly when the CLI would exit `1` (ARC-5.4).
    pub(crate) is_error: bool,
}

/// Why a tool call gave no result: the usage error of its arguments, or an output document
/// that could not be written (the CLI exits `2` for both).
#[derive(Debug)]
pub(crate) enum CallFailure {
    /// The arguments are refused (ARC-6.7).
    Usage(UsageError),
    /// The output document could not be written as JSON.
    Output(OutputError),
}

impl From<UsageError> for CallFailure {
    fn from(error: UsageError) -> Self {
        Self::Usage(error)
    }
}

impl From<OutputError> for CallFailure {
    fn from(error: OutputError) -> Self {
        Self::Output(error)
    }
}

/// Runs `call` and returns its result.
///
/// # Errors
///
/// [`CallFailure`] when an option or the dialect is refused, or a document cannot be written.
pub(crate) fn run_call(call: &ToolCall) -> Result<ToolOutput, CallFailure> {
    match call.command {
        CommandName::Validate => run_validate(call),
        CommandName::Compile => run_compile(call),
        CommandName::Explain => run_explain(call),
        CommandName::Describe => run_describe(call),
        CommandName::IntegrityChecks => run_integrity_checks(call),
        CommandName::Dependencies => run_dependencies(call),
        CommandName::Migrate => run_migrate(call),
        CommandName::Diff => run_diff(call),
    }
}

/// A successful document.
fn success(text: String) -> ToolOutput {
    ToolOutput {
        texts: vec![text],
        is_error: false,
    }
}

/// A compile error: the CLI exits `1`.
fn failure(error: &CompileErrorDocument) -> Result<ToolOutput, CallFailure> {
    Ok(ToolOutput {
        texts: vec![document_json(error)?],
        is_error: true,
    })
}

/// The load options of the call, or the defaults (ARC-5.5).
fn load_options(call: &ToolCall) -> Result<LoadOptions, UsageError> {
    call.options(Argument::LoadOptions)
        .map_or_else(|| Ok(LoadOptions::default()), parse_load_options)
}

/// The compile options of the call, or the defaults (ARC-5.7).
fn compile_options(call: &ToolCall) -> Result<CompileOptions, UsageError> {
    call.options(Argument::CompileOptions)
        .map_or_else(|| Ok(CompileOptions::default()), parse_compile_options)
}

/// Loads the model of `model` against the catalog schema of `catalog_schema`, with the catalog
/// state when the tool takes one and the call gives it.
fn load_model(
    call: &ToolCall,
    model: Argument,
    catalog_schema: Argument,
    options: &LoadOptions,
) -> Result<Result<Handle, CompileErrorDocument>, UsageError> {
    let state = if call
        .command
        .optional_arguments()
        .contains(&Argument::CatalogState)
    {
        call.bytes(Argument::CatalogState)
    } else {
        None
    };
    Ok(load(
        call.required(model)?,
        call.required(catalog_schema)?,
        state,
        options,
    ))
}

/// Loads the handle of `compile`, `explain` and `describe` (ARC-5.7, ARC-5.8).
fn load_handle(call: &ToolCall) -> Result<Result<Handle, CompileErrorDocument>, UsageError> {
    let options = load_options(call)?;
    load_model(call, Argument::Model, Argument::CatalogSchema, &options)
}

fn run_validate(call: &ToolCall) -> Result<ToolOutput, CallFailure> {
    let options = load_options(call)?;
    let report = validate(
        call.required(Argument::Model)?,
        call.required(Argument::CatalogSchema)?,
        &options,
    );
    Ok(ToolOutput {
        texts: vec![document_json(&report)?],
        is_error: !report.errors.is_empty(),
    })
}

/// `compile`: the plan or the compile error, then the audit record when `audit` is true and
/// `compile` ran (ARC-6.8).
fn run_compile(call: &ToolCall) -> Result<ToolOutput, CallFailure> {
    let options = compile_options(call)?;
    let handle = match load_handle(call)? {
        Ok(handle) => handle,
        Err(error) => return failure(&error),
    };
    let compilation = compile(
        &handle,
        call.required(Argument::Query)?,
        call.required(Argument::SecurityContext)?,
        &options,
    );
    let mut output = compile_output(compilation.result)?;
    if let Some(record) = &compilation.audit {
        output.texts.push(audit_record_json(record)?);
    }
    Ok(output)
}

fn run_explain(call: &ToolCall) -> Result<ToolOutput, CallFailure> {
    let options = compile_options(call)?;
    let handle = match load_handle(call)? {
        Ok(handle) => handle,
        Err(error) => return failure(&error),
    };
    compile_output(explain(
        &handle,
        call.required(Argument::Query)?,
        call.required(Argument::SecurityContext)?,
        &options,
    ))
}

fn compile_output(result: CompileResult) -> Result<ToolOutput, CallFailure> {
    match result {
        CompileResult::Plan(plan) => Ok(success(plan_json(&plan)?)),
        CompileResult::Error(error) => failure(&error),
    }
}

fn run_describe(call: &ToolCall) -> Result<ToolOutput, CallFailure> {
    let handle = match load_handle(call)? {
        Ok(handle) => handle,
        Err(error) => return failure(&error),
    };
    match describe(&handle, call.required(Argument::SecurityContext)?) {
        DescribeResult::Description(document) => Ok(success(description_json(&document)?)),
        DescribeResult::Error(error) => failure(&error),
    }
}

fn run_integrity_checks(call: &ToolCall) -> Result<ToolOutput, CallFailure> {
    let dialect = call
        .text(Argument::Dialect)
        .map_or(Ok(Dialect::Duckdb), parse_dialect)?;
    let options = load_options(call)?;
    let handle = match load_model(call, Argument::Model, Argument::CatalogSchema, &options)? {
        Ok(handle) => handle,
        Err(error) => return failure(&error),
    };
    match integrity_checks(&handle, dialect) {
        Ok(document) => Ok(success(integrity_checks_json(&document)?)),
        Err(error) => failure(&error),
    }
}

fn run_dependencies(call: &ToolCall) -> Result<ToolOutput, CallFailure> {
    let options = load_options(call)?;
    let handle = match load_model(call, Argument::Model, Argument::CatalogSchema, &options)? {
        Ok(handle) => handle,
        Err(error) => return failure(&error),
    };
    match dependencies(&handle) {
        Ok(document) => Ok(success(dependencies_json(&document)?)),
        Err(error) => failure(&error),
    }
}

fn run_migrate(call: &ToolCall) -> Result<ToolOutput, CallFailure> {
    let options = load_options(call)?;
    match migrate(call.required(Argument::Model)?, &options) {
        Ok(report) => Ok(success(migration_report_json(&report)?)),
        Err(error) => failure(&error),
    }
}

/// `diff`: both models with the same load options, old first (ARC-5.6, DEP-4.4).
fn run_diff(call: &ToolCall) -> Result<ToolOutput, CallFailure> {
    let options = load_options(call)?;
    let old = match load_model(
        call,
        Argument::OldModel,
        Argument::OldCatalogSchema,
        &options,
    )? {
        Ok(handle) => handle,
        Err(error) => return failure(&error),
    };
    let new = match load_model(
        call,
        Argument::NewModel,
        Argument::NewCatalogSchema,
        &options,
    )? {
        Ok(handle) => handle,
        Err(error) => return failure(&error),
    };
    match dependencies(&old).and_then(|old| dependencies(&new).and_then(|new| diff(&old, &new))) {
        Ok(document) => Ok(success(diff_json(&document)?)),
        Err(error) => failure(&error),
    }
}

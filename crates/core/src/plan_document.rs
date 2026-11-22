//! The compiled plan document (PLN-1), with its lineage (PLN-4) and the outputs of VS-5.

use menura_contracts::ContractKind;
use menura_contracts::types::common::Decision;
use menura_contracts::types::compiled_plan::{
    CompiledPlanContract, CompiledPlanDocument, Diagnostic, Limits,
};
use menura_generator_duckdb::GeneratedSql;
use menura_representations::bound_query::BoundQuery;
use menura_representations::logical_plan::{Operation, PlanNode};
use menura_security::AdmittedSecurityContext;

use crate::cache_key::{HashedPlan, cache_key};
use crate::compile::InternalError;
use crate::compile_options::CompileOptions;
use crate::handle::Handle;
use crate::lineage::{LineageInputs, lineage};
use crate::producer::producer;

/// The members of the compiled plan that the stages gathered, besides the SQL.
pub(crate) struct PlanOutputs {
    /// The explanation, assembled in step order (PLN-7).
    pub(crate) explanation: Vec<Decision>,
    /// The warnings.
    pub(crate) warnings: Vec<Diagnostic>,
}

/// The compiled plan of `plan`, the final plan of `query`, whose SQL is `generated`, for the
/// user of `context`, with its lineage (PLN-4) and the explanation and warnings of `outputs`.
///
/// # Errors
///
/// An [`InternalError`] when the plan scans a dataset the model does not have, or names an
/// element its lineage cannot derive.
pub(crate) fn plan_document(
    handle: &Handle,
    plan: &PlanNode,
    query: &BoundQuery,
    generated: GeneratedSql,
    context: &AdmittedSecurityContext,
    options: &CompileOptions,
    outputs: PlanOutputs,
) -> Result<CompiledPlanDocument, InternalError> {
    let data_source = data_source(handle);
    let sources = relations_read(handle, plan)?;
    let lineage = lineage(
        plan,
        query,
        LineageInputs {
            model: handle.model(),
            catalog: handle.catalog_schema(),
            context,
        },
    )?;
    let cache_key = cache_key(
        &HashedPlan {
            dialect: options.dialect,
            data_source: &data_source,
            sql: &generated.sql,
            parameters: &generated.parameters,
            result_schema: &generated.result_schema,
        },
        context.fingerprint(),
        handle.catalog_state(),
        &sources,
    );
    Ok(CompiledPlanDocument {
        contract: CompiledPlanContract::CompiledPlan,
        contract_version: ContractKind::CompiledPlan.latest_version(),
        producer: producer()?,
        dialect: options.dialect,
        data_source,
        sql: generated.sql,
        parameters: generated.parameters,
        result_schema: generated.result_schema,
        cache_key,
        limits: Limits {
            timeout_ms: options.timeout_ms.get(),
            max_rows: options.max_rows.map(std::num::NonZeroU64::get),
        },
        lineage,
        explanation: outputs.explanation,
        warnings: outputs.warnings,
    })
}

/// The `data_source` load option, or the catalog schema's `document_id.origin`.
fn data_source(handle: &Handle) -> String {
    handle
        .load_options()
        .data_source
        .clone()
        .unwrap_or_else(|| handle.catalog_schema().document_id.origin.clone())
}

/// The catalog relation of every dataset `plan` scans, in plan order without repetition.
fn relations_read<'handle>(
    handle: &'handle Handle,
    plan: &PlanNode,
) -> Result<Vec<&'handle str>, InternalError> {
    let mut sources = Vec::new();
    let mut pending = vec![plan];
    while let Some(node) = pending.pop() {
        if let Operation::Scan { dataset, .. } = node.operation() {
            let Some(found) = handle.model().dataset(dataset) else {
                return Err(InternalError::new(
                    "core",
                    format!(
                        "the plan scans `{}`, which the model does not have",
                        dataset.as_str()
                    ),
                ));
            };
            if !sources.contains(&found.source.as_str()) {
                sources.push(found.source.as_str());
            }
        }
        pending.extend(node.inputs().into_iter().rev());
    }
    Ok(sources)
}

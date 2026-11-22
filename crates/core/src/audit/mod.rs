//! The audit record `compile` returns with `audit: true` (PLN-6).

mod authorization;

use std::collections::BTreeMap;

use menura_contracts::ContractKind;
use menura_contracts::types::audit_record::{
    AuditInputs, AuditRecordContract, AuditRecordDocument, CompileOptions as AuditCompileOptions,
    ErrorOutcome, LoadOptions, Outcome, PlanOutcome,
};
use menura_contracts::types::common::Decision;
use menura_representations::bound_query::AuthorizationDecision;
use menura_security::AdmittedSecurityContext;

use crate::compile::InternalError;
use crate::compile_options::CompileOptions;
use crate::handle::Handle;
use crate::passed_document::PassedDocument;
use crate::plan_document::data_source;
use crate::producer::{OSSIE_NORMATIVE, producer};
use authorization::authorization_entries;

/// What one compilation recorded for its audit record, besides the handle and the options.
pub(crate) struct AuditFacts<'facts> {
    /// The query document as passed.
    pub(crate) query: &'facts PassedDocument,
    /// The security context document as passed.
    pub(crate) security_context: &'facts PassedDocument,
    /// The admitted security context, or `None` when it was rejected (PLN-6.4).
    pub(crate) context: Option<&'facts AdmittedSecurityContext>,
    /// The plan hash or the error codes.
    pub(crate) outcome: Outcome,
    /// Every authorization decision of E2, in order (E2-4.7).
    pub(crate) authorization: &'facts [AuthorizationDecision],
    /// Every row policy applied (PLN-6.6).
    pub(crate) row_policies: Vec<String>,
    /// The explanation entries, in plan order (PLN-6.6).
    pub(crate) decisions: Vec<Decision>,
}

/// The audit record of a compilation over `handle` with `options` (PLN-6). It holds no data
/// value and no attribute value (PLN-6.1).
///
/// # Errors
///
/// An [`InternalError`] when the producer of the build cannot be written.
pub(crate) fn audit_record(
    handle: &Handle,
    options: &CompileOptions,
    facts: AuditFacts<'_>,
) -> Result<AuditRecordDocument, InternalError> {
    let contract_versions = contract_versions(handle, &facts);
    Ok(AuditRecordDocument {
        contract: AuditRecordContract::AuditRecord,
        contract_version: ContractKind::AuditRecord.latest_version(),
        subject: facts.context.map(|context| context.subject().to_owned()),
        inputs: AuditInputs {
            model: handle.model().hash().clone(),
            catalog_schema: handle.passed_schema().hash.clone(),
            catalog_state: handle.passed_state().map(|state| state.hash.clone()),
            query: facts.query.hash.clone(),
            security_fingerprint: facts.context.map(AdmittedSecurityContext::fingerprint),
        },
        producer: producer()?,
        outcome: facts.outcome,
        authorization: authorization_entries(facts.authorization),
        row_policies: facts.row_policies,
        decisions: facts.decisions,
        load_options: load_options(handle),
        compile_options: Some(AuditCompileOptions {
            dialect: options.dialect,
        }),
        contract_versions,
        actor: options.actor.clone(),
    })
}

/// The outcome of a compiled plan whose `plan_hash` is `plan_hash`.
pub(crate) fn plan_outcome(plan_hash: menura_contracts::Sha256Hash) -> Outcome {
    Outcome::Plan(PlanOutcome { plan_hash })
}

/// The outcome of a compile error whose errors have the codes `codes`.
pub(crate) fn error_outcome(codes: Vec<String>) -> Outcome {
    Outcome::Error(ErrorOutcome { error_codes: codes })
}

/// Every load option the handle uses, defaults included; `model_name` only when it was
/// passed, and `data_source` as every plan writes it (PLN-6.7).
fn load_options(handle: &Handle) -> LoadOptions {
    LoadOptions {
        data_source: Some(data_source(handle)),
        ..handle.load_options().stage_options()
    }
}

/// The version of every input contract, by input name: the model's Apache Ossie `version`,
/// which E0B-01 requires to be the normative release, and the `contract_version` of every
/// other document as passed, when it has one.
fn contract_versions(handle: &Handle, facts: &AuditFacts<'_>) -> BTreeMap<String, String> {
    let documents = [
        ("catalog_schema", Some(handle.passed_schema())),
        ("catalog_state", handle.passed_state()),
        ("query", Some(facts.query)),
        ("security_context", Some(facts.security_context)),
    ];
    let mut versions = BTreeMap::from([("model".to_owned(), OSSIE_NORMATIVE.to_owned())]);
    for (input, document) in documents {
        if let Some(version) = document.and_then(|document| document.contract_version.clone()) {
            versions.insert(input.to_owned(), version);
        }
    }
    versions
}

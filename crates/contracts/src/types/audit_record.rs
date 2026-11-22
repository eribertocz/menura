//! The audit record document (PLN-6).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::common::{Decision, Dialect, Producer};
use crate::contract::ContractVersion;
use crate::hash::Sha256Hash;

/// The `contract` value `menura.audit_record`, or its former name (CON-0.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditRecordContract {
    /// The name the compiler writes (CON-0.5).
    #[serde(rename = "menura.audit_record")]
    AuditRecord,
    /// The former name, read until 1.0.0 and never written (CON-0.5).
    #[serde(rename = "semantic_compiler.audit_record")]
    FormerAuditRecord,
}

/// An audit record of one compilation. It holds no data values and no attribute values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditRecordDocument {
    /// `menura.audit_record`, or its former name.
    pub contract: AuditRecordContract,
    /// Contract version.
    pub contract_version: ContractVersion,
    /// Subject of the security context, or `null` when the context was not admitted (PLN-6.4).
    pub subject: Option<String>,
    /// Input hashes.
    pub inputs: AuditInputs,
    /// Producer versions.
    pub producer: Producer,
    /// Plan hash or error codes.
    pub outcome: Outcome,
    /// Every authorization decision.
    pub authorization: Vec<AuthorizationDecision>,
    /// Every row policy applied.
    pub row_policies: Vec<String>,
    /// Every decision of the explanation.
    pub decisions: Vec<Decision>,
    /// Load options.
    pub load_options: LoadOptions,
    /// The compile options that shape the plan (PLN-6.7); absent before contract version 1.2.0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compile_options: Option<CompileOptions>,
    /// Version of every input contract, by input name.
    pub contract_versions: BTreeMap<String, String>,
    /// The delegated actor the host passed as a compile option, written unchanged (PLN-6.8);
    /// absent when the option is not passed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
}

/// Hashes of the inputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditInputs {
    /// Model hash.
    pub model: Sha256Hash,
    /// Catalog schema hash.
    pub catalog_schema: Sha256Hash,
    /// Catalog state hash, or `null` without a state.
    pub catalog_state: Option<Sha256Hash>,
    /// Query hash.
    pub query: Sha256Hash,
    /// Security context fingerprint, or `null` when the context was not admitted (PLN-6.4).
    pub security_fingerprint: Option<Sha256Hash>,
}

/// The outcome of the compilation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Outcome {
    /// A plan was produced.
    Plan(PlanOutcome),
    /// Compilation failed.
    Error(ErrorOutcome),
}

/// A plan was produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanOutcome {
    /// Plan hash (PLN-3.1).
    pub plan_hash: Sha256Hash,
}

/// Compilation failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorOutcome {
    /// Error codes, at least one.
    pub error_codes: Vec<String>,
}

/// Whether access was granted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Access {
    /// Granted.
    Granted,
    /// Denied.
    Denied,
}

/// One authorization decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorizationDecision {
    /// Object decided on.
    pub object: String,
    /// Granted or denied.
    pub decision: Access,
    /// The rule that decided.
    pub rule: String,
}

/// Format of the Apache Ossie model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OssieFormat {
    /// The normative release.
    Release,
    /// The tracked development line.
    Development,
}

/// The load options of the model handle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoadOptions {
    /// DEV-4.
    pub grant_authenticated_to_all: bool,
    /// Model format.
    pub ossie_format: OssieFormat,
    /// Extension namespace.
    pub namespace: String,
    /// Model selected, when the document has several.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    /// The logical source name written in every plan (PLN-6.7); absent before contract version
    /// 1.2.0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_source: Option<String>,
}

/// The compile options that shape the plan (PLN-6.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompileOptions {
    /// Target dialect.
    pub dialect: Dialect,
}

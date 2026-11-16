//! The audit record document (PLN-6).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::common::{Decision, Producer};
use crate::contract::ContractVersion;
use crate::hash::Sha256Hash;

/// The `contract` value `semantic_compiler.audit_record`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditRecordContract {
    /// The only value.
    #[serde(rename = "semantic_compiler.audit_record")]
    AuditRecord,
}

/// An audit record of one compilation. It holds no data values and no attribute values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditRecordDocument {
    /// Always `semantic_compiler.audit_record`.
    pub contract: AuditRecordContract,
    /// Contract version.
    pub contract_version: ContractVersion,
    /// Subject of the security context.
    pub subject: String,
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
    /// Version of every input contract, by input name.
    pub contract_versions: BTreeMap<String, String>,
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
    /// Security context fingerprint.
    pub security_fingerprint: Sha256Hash,
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
}

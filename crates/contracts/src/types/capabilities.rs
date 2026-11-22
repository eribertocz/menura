//! The capability profile of a compiler release, part of the model description (DSC-4).

use serde::{Deserialize, Serialize};

use super::audit_record::OssieFormat;
use crate::contract::ContractVersion;

/// What the compiler release supports (DSC-4): a constant of the release, independent of the
/// model and of the security context (DSC-4.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    /// Compiler version.
    pub compiler_version: ContractVersion,
    /// The roadmap slice the release implements.
    pub slice_version: ContractVersion,
    /// The Apache Ossie releases and formats it reads.
    pub ossie: OssieSupport,
    /// Every contract it reads or writes, with its latest version, sorted by `contract`.
    pub contracts: Vec<SupportedContract>,
    /// SQL dialects it generates.
    pub dialects: Vec<String>,
    /// Query forms it compiles.
    pub query_forms: Vec<QueryFormName>,
    /// Extensions it reads, sorted by `key`.
    pub extensions: Vec<SupportedExtension>,
    /// Public functions it provides.
    pub functions: Vec<CompilerFunction>,
    /// Limits of the release.
    pub limits: CapabilityLimits,
}

/// The Apache Ossie support of the release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OssieSupport {
    /// Normative Apache Ossie release.
    pub normative: String,
    /// Tracked Apache Ossie development commit (7–40 lowercase hex digits).
    pub tracked_commit: String,
    /// Model formats read.
    pub formats: Vec<OssieFormat>,
}

/// One supported contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportedContract {
    /// Contract name, `menura.<name>`.
    pub contract: String,
    /// Latest supported version.
    pub version: ContractVersion,
}

/// A query form the release compiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryFormName {
    /// The SQL form.
    Sql,
    /// The wide-table form.
    WideTable,
}

/// One supported extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportedExtension {
    /// Extension key.
    pub key: String,
    /// Supported versions, ascending.
    pub versions: Vec<u64>,
}

/// A public function of the compiler (architecture section 3.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompilerFunction {
    /// `compile`.
    Compile,
    /// `describe`.
    Describe,
    /// `explain`.
    Explain,
    /// `integrity_checks`.
    IntegrityChecks,
    /// `load`.
    Load,
    /// `migrate`.
    Migrate,
    /// `validate`.
    Validate,
    /// `with_state`.
    WithState,
}

/// Limits of the release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityLimits {
    /// Longest identifier accepted.
    pub max_identifier_length: u64,
    /// Largest decimal precision accepted.
    pub max_decimal_precision: u64,
}

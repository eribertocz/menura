//! The public contracts (CON-0.1) and their supported versions (CAT-3.2).

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::schema::strings::is_semantic_version;

/// Every public contract document with a JSON Schema in this repository (CON-0.1).
///
/// The Apache Ossie semantic model is also public but follows the Apache Ossie schema, so it is
/// not listed here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContractKind {
    /// `semantic_compiler.catalog_schema` (in).
    CatalogSchema,
    /// `semantic_compiler.catalog_state` (in).
    CatalogState,
    /// `semantic_compiler.query` (in).
    Query,
    /// `semantic_compiler.security_context` (in).
    SecurityContext,
    /// `semantic_compiler.compiled_plan` (out).
    CompiledPlan,
    /// `semantic_compiler.compile_error` (out).
    CompileError,
    /// `semantic_compiler.audit_record` (out).
    AuditRecord,
    /// `semantic_compiler.model_description` (out).
    ModelDescription,
    /// `semantic_compiler.integrity_checks` (out).
    IntegrityChecks,
}

impl ContractKind {
    /// Every contract, in the order of the contracts table.
    pub const ALL: [Self; 9] = [
        Self::CatalogSchema,
        Self::CatalogState,
        Self::Query,
        Self::SecurityContext,
        Self::CompiledPlan,
        Self::CompileError,
        Self::AuditRecord,
        Self::ModelDescription,
        Self::IntegrityChecks,
    ];

    /// The value of the document's `contract` member.
    #[must_use]
    pub fn contract_name(self) -> &'static str {
        match self {
            Self::CatalogSchema => "semantic_compiler.catalog_schema",
            Self::CatalogState => "semantic_compiler.catalog_state",
            Self::Query => "semantic_compiler.query",
            Self::SecurityContext => "semantic_compiler.security_context",
            Self::CompiledPlan => "semantic_compiler.compiled_plan",
            Self::CompileError => "semantic_compiler.compile_error",
            Self::AuditRecord => "semantic_compiler.audit_record",
            Self::ModelDescription => "semantic_compiler.model_description",
            Self::IntegrityChecks => "semantic_compiler.integrity_checks",
        }
    }

    /// The latest contract version this crate implements.
    #[must_use]
    pub fn latest_version(self) -> ContractVersion {
        ContractVersion {
            major: 1,
            minor: 0,
            patch: 0,
        }
    }

    /// Whether `version` is accepted (CAT-3.2): its major version is the known one and its minor
    /// version is at most the latest known minor version.
    #[must_use]
    pub fn accepts_version(self, version: ContractVersion) -> bool {
        let latest = self.latest_version();
        version.major == latest.major && version.minor <= latest.minor
    }

    /// Whether `document` names this contract and an accepted `contract_version`.
    /// A missing or non-string member is unsupported.
    #[must_use]
    pub fn supports(self, document: &Value) -> bool {
        let names_contract =
            document.get("contract").and_then(Value::as_str) == Some(self.contract_name());
        let version = document
            .get("contract_version")
            .and_then(Value::as_str)
            .and_then(ContractVersion::parse);
        names_contract && version.is_some_and(|version| self.accepts_version(version))
    }
}

impl fmt::Display for ContractKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.contract_name())
    }
}

/// A contract version, `MAJOR.MINOR.PATCH` (semantic versioning), serialized as that string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ContractVersion {
    /// Incompatible changes.
    pub major: u64,
    /// Compatible additions.
    pub minor: u64,
    /// Fixes.
    pub patch: u64,
}

impl ContractVersion {
    /// Parses `MAJOR.MINOR.PATCH` with no leading zeros. Returns `None` for any other text.
    ///
    /// ```
    /// use semantic_compiler_contracts::ContractVersion;
    ///
    /// assert_eq!(ContractVersion::parse("1.2.3").map(|version| version.minor), Some(2));
    /// assert_eq!(ContractVersion::parse("1.02.3"), None);
    /// ```
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        if !is_semantic_version(text) {
            return None;
        }
        let mut parts = text.split('.').map(str::parse::<u64>);
        let (Some(Ok(major)), Some(Ok(minor)), Some(Ok(patch))) =
            (parts.next(), parts.next(), parts.next())
        else {
            return None;
        };
        Some(Self {
            major,
            minor,
            patch,
        })
    }
}

impl fmt::Display for ContractVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl TryFrom<String> for ContractVersion {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::parse(&text).ok_or_else(|| format!("`{text}` is not a semantic version"))
    }
}

impl From<ContractVersion> for String {
    fn from(version: ContractVersion) -> Self {
        version.to_string()
    }
}

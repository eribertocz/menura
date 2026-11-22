//! The public contracts (CON-0.1), their names (CON-0.5) and their supported versions
//! (CAT-3.2).

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::schema::strings::is_semantic_version;

/// The namespace of every contract name and of the compiler's extension entries (CON-0.5,
/// MDL-2.3).
pub const NAMESPACE: &str = "menura";

/// The former namespace, still read until 1.0.0 (CON-0.5): in the `contract` member of an
/// input document, and as an extension namespace (E0C-1, condition 3).
pub const FORMER_NAMESPACE: &str = "semantic_compiler";

/// Every public contract document with a JSON Schema in this repository (CON-0.1).
///
/// The Apache Ossie semantic model is also public but follows the Apache Ossie schema, so it is
/// not listed here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContractKind {
    /// `menura.catalog_schema` (in).
    CatalogSchema,
    /// `menura.catalog_state` (in).
    CatalogState,
    /// `menura.query` (in).
    Query,
    /// `menura.security_context` (in).
    SecurityContext,
    /// `menura.compiled_plan` (out).
    CompiledPlan,
    /// `menura.compile_error` (out).
    CompileError,
    /// `menura.audit_record` (out).
    AuditRecord,
    /// `menura.model_description` (out).
    ModelDescription,
    /// `menura.integrity_checks` (out).
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

    /// The part of the contract name after the namespace, such as `catalog_schema`.
    #[must_use]
    pub fn local_name(self) -> &'static str {
        match self {
            Self::CatalogSchema => "catalog_schema",
            Self::CatalogState => "catalog_state",
            Self::Query => "query",
            Self::SecurityContext => "security_context",
            Self::CompiledPlan => "compiled_plan",
            Self::CompileError => "compile_error",
            Self::AuditRecord => "audit_record",
            Self::ModelDescription => "model_description",
            Self::IntegrityChecks => "integrity_checks",
        }
    }

    /// The value of the `contract` member the compiler writes: `menura.<name>` (CON-0.5).
    #[must_use]
    pub fn contract_name(self) -> &'static str {
        match self {
            Self::CatalogSchema => "menura.catalog_schema",
            Self::CatalogState => "menura.catalog_state",
            Self::Query => "menura.query",
            Self::SecurityContext => "menura.security_context",
            Self::CompiledPlan => "menura.compiled_plan",
            Self::CompileError => "menura.compile_error",
            Self::AuditRecord => "menura.audit_record",
            Self::ModelDescription => "menura.model_description",
            Self::IntegrityChecks => "menura.integrity_checks",
        }
    }

    /// The former value of the `contract` member, `semantic_compiler.<name>`, read exactly as
    /// [`Self::contract_name`] until 1.0.0 (CON-0.5).
    #[must_use]
    pub fn former_contract_name(self) -> &'static str {
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

    /// Whether `document` names this contract, with its name or its former name (CON-0.5).
    /// A missing or non-string member names no contract.
    #[must_use]
    pub fn is_named_by(self, document: &Value) -> bool {
        document
            .get("contract")
            .and_then(Value::as_str)
            .is_some_and(|name| name == self.contract_name() || name == self.former_contract_name())
    }

    /// Whether `document` names this contract with its former name (CON-0.5).
    #[must_use]
    pub fn is_named_by_former_name(self, document: &Value) -> bool {
        document.get("contract").and_then(Value::as_str) == Some(self.former_contract_name())
    }

    /// The latest version of this contract the crate implements (architecture section 3.3):
    /// 1.2.0 for the compiled plan, the compile error, the audit record, the model description
    /// and the integrity checks, 1.1.0 for the other contracts.
    ///
    /// ```
    /// use menura_contracts::ContractKind;
    ///
    /// assert_eq!(ContractKind::CompiledPlan.latest_version().to_string(), "1.2.0");
    /// assert_eq!(ContractKind::Query.latest_version().to_string(), "1.1.0");
    /// ```
    #[must_use]
    pub fn latest_version(self) -> ContractVersion {
        let minor = match self {
            Self::CompiledPlan
            | Self::CompileError
            | Self::AuditRecord
            | Self::ModelDescription
            | Self::IntegrityChecks => 2,
            Self::CatalogSchema | Self::CatalogState | Self::Query | Self::SecurityContext => 1,
        };
        ContractVersion {
            major: 1,
            minor,
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

    /// Whether `document` names this contract, with its name or its former name (CON-0.5),
    /// and an accepted `contract_version`. A missing or non-string member is unsupported.
    #[must_use]
    pub fn supports(self, document: &Value) -> bool {
        let names_contract = self.is_named_by(document);
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
    /// use menura_contracts::ContractVersion;
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

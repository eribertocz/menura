//! The migration report (MIG-2): the model with its deprecated forms rewritten, and each
//! rewrite.

use serde::{Deserialize, Serialize};

use super::common::Producer;
use super::ordered_json::OrderedObject;
use crate::contract::ContractVersion;

/// The `contract` value `menura.migration_report`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationReportContract {
    /// The name the compiler writes.
    #[serde(rename = "menura.migration_report")]
    MigrationReport,
}

/// The output of `migrate` (MIG-2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationReportDocument {
    /// `menura.migration_report`.
    pub contract: MigrationReportContract,
    /// Contract version.
    pub contract_version: ContractVersion,
    /// The compiler build, as in the compiled plan (PLN-1).
    pub producer: Producer,
    /// One change per rewrite, ordered by `pointer` in document order (MIG-3.4).
    pub changes: Vec<MigrationChange>,
    /// The migrated model document, its members in the order they were read (MIG-3.3).
    pub model: OrderedObject,
}

/// One rewrite (MIG-3.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationChange {
    /// The RFC 6901 pointer of the rewritten member in `model`.
    pub pointer: String,
    /// The rewrite rule.
    pub rule: MigrationRule,
    /// The value before the rewrite.
    pub from: String,
    /// The value after the rewrite.
    pub to: String,
}

/// The rewrite rules of this version (MIG-3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MigrationRule {
    /// `MIG-3.2a`: the former extension namespace `semantic_compiler`.
    #[serde(rename = "MIG-3.2a")]
    FormerNamespace,
}

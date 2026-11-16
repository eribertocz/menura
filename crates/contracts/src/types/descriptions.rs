//! The model description (DSC-2) and integrity checks (CHK-2) documents.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::common::{Dialect, Grain, LogicalType, Parameter, present};
use crate::contract::ContractVersion;
use crate::hash::Sha256Hash;

/// The `contract` value `semantic_compiler.model_description`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelDescriptionContract {
    /// The only value.
    #[serde(rename = "semantic_compiler.model_description")]
    ModelDescription,
}

/// What one security context may ask of one model (DSC-2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelDescriptionDocument {
    /// Always `semantic_compiler.model_description`.
    pub contract: ModelDescriptionContract,
    /// Contract version.
    pub contract_version: ContractVersion,
    /// Model the description is valid for.
    pub model_hash: Sha256Hash,
    /// Security context the description is valid for.
    pub security_fingerprint: Sha256Hash,
    /// Authorized datasets.
    pub datasets: Vec<DescribedDataset>,
    /// Authorized metrics.
    pub metrics: Vec<DescribedMetric>,
    /// Dataset pairs with more than one path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ambiguous_paths: Option<Vec<AmbiguousPath>>,
}

/// An authorized dataset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DescribedDataset {
    /// Dataset name.
    pub name: String,
    /// Label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Authorized fields.
    pub fields: Vec<DescribedField>,
}

/// An authorized field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DescribedField {
    /// `dataset.field`.
    pub reference: String,
    /// Label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Logical type.
    pub logical_type: LogicalType,
    /// Whether it is a time field.
    pub is_time: bool,
    /// Grains it accepts.
    pub time_grains: Vec<Grain>,
    /// Apache Ossie `ai_context`, any JSON value.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub ai_context: Option<Value>,
}

/// Kind of metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricKind {
    /// Aggregates fields.
    Base,
    /// Combines other metrics.
    Derived,
    /// Evaluated in another context.
    Context,
}

/// An authorized metric.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DescribedMetric {
    /// Metric reference.
    pub reference: String,
    /// Description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Result type.
    pub logical_type: LogicalType,
    /// Kind.
    pub kind: MetricKind,
    /// Datasets reachable from it (E3-3.4).
    pub reachable_datasets: Vec<String>,
    /// Apache Ossie `ai_context`, any JSON value.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub ai_context: Option<Value>,
}

/// A dataset pair with more than one path, with two witness paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmbiguousPath {
    /// Starting dataset.
    pub from: String,
    /// Ending dataset.
    pub to: String,
    /// Witness paths, each a list of relationships.
    pub paths: Vec<Vec<String>>,
}

/// The `contract` value `semantic_compiler.integrity_checks`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntegrityChecksContract {
    /// The only value.
    #[serde(rename = "semantic_compiler.integrity_checks")]
    IntegrityChecks,
}

/// The integrity checks generated for a model (CHK-2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegrityChecksDocument {
    /// Always `semantic_compiler.integrity_checks`.
    pub contract: IntegrityChecksContract,
    /// Contract version.
    pub contract_version: ContractVersion,
    /// Model the checks were generated from.
    pub model_hash: Sha256Hash,
    /// Dialect of the check queries.
    pub dialect: Dialect,
    /// The checks.
    pub checks: Vec<IntegrityCheck>,
}

/// Kind of integrity check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckKind {
    /// Relationship key uniqueness.
    KeyUniqueness,
    /// Foreign keys without a match.
    OrphanKeys,
    /// Summary consistency.
    Summary,
    /// Population extension.
    Population,
    /// Semi-additive metric extension.
    SemiAdditive,
}

/// One integrity check (CHK-2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegrityCheck {
    /// `<kind>:<target>`.
    pub check_id: String,
    /// Kind.
    pub kind: CheckKind,
    /// Relationship, dataset, summary or metric name.
    pub target: String,
    /// Definition hash (CHK-3).
    pub definition_hash: Sha256Hash,
    /// Read-only query; it passes when it returns zero rows.
    pub sql: String,
    /// Bound parameters.
    pub parameters: Vec<Parameter>,
    /// Catalog relations it reads.
    pub sources: Vec<String>,
}

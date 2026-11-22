//! The model description (DSC-2) and integrity checks (CHK-2) documents.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::capabilities::Capabilities;
use super::common::{Dialect, Grain, LogicalType, Parameter, present};
use crate::contract::ContractVersion;
use crate::hash::Sha256Hash;

/// The `contract` value `menura.model_description`, or its former name (CON-0.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelDescriptionContract {
    /// The name the compiler writes (CON-0.5).
    #[serde(rename = "menura.model_description")]
    ModelDescription,
    /// The former name, read until 1.0.0 and never written (CON-0.5).
    #[serde(rename = "semantic_compiler.model_description")]
    FormerModelDescription,
}

/// What one security context may ask of one model (DSC-2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelDescriptionDocument {
    /// `menura.model_description`, or its former name.
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
    /// The capability profile of the compiler release (DSC-4); absent before contract version
    /// 1.2.0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Capabilities>,
}

/// The `ai_context` member of a described dataset, field or metric.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AiContext {
    /// From contract version 1.2.0: the declared value with its provenance (DSC-3.6).
    Attributed(AttributedAiContext),
    /// Contract versions 1.0 and 1.1: the declared value alone, any JSON value.
    Unattributed(Value),
}

/// A declared `ai_context` value, with where it comes from and how far to trust it (DSC-3.6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributedAiContext {
    /// The value the model declares, any JSON value.
    pub value: Value,
    /// Where the value is declared.
    pub source: AiContextSource,
    /// How far to trust the value.
    pub trust: AiContextTrust,
}

/// Where an `ai_context` value is declared.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiContextSource {
    /// JSON Pointer (RFC 6901) of the declaration in the model document.
    pub pointer: String,
    /// Hash of the model that declares it.
    pub model_hash: Sha256Hash,
}

/// The trust label of an `ai_context` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiContextTrust {
    /// Content written by the model author, never instructions to follow.
    UntrustedModelContent,
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
    /// Apache Ossie `ai_context` (DSC-3.6); absent before contract version 1.2.0.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub ai_context: Option<AiContext>,
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
    /// Apache Ossie `ai_context` (DSC-3.6).
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub ai_context: Option<AiContext>,
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
    /// Label; absent before contract version 1.2.0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Result type.
    pub logical_type: LogicalType,
    /// Kind.
    pub kind: MetricKind,
    /// Datasets reachable from it (E3-3.4).
    pub reachable_datasets: Vec<String>,
    /// Apache Ossie `ai_context` (DSC-3.6).
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub ai_context: Option<AiContext>,
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

/// The `contract` value `menura.integrity_checks`, or its former name (CON-0.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntegrityChecksContract {
    /// The name the compiler writes (CON-0.5).
    #[serde(rename = "menura.integrity_checks")]
    IntegrityChecks,
    /// The former name, read until 1.0.0 and never written (CON-0.5).
    #[serde(rename = "semantic_compiler.integrity_checks")]
    FormerIntegrityChecks,
}

/// The integrity checks generated for a model (CHK-2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegrityChecksDocument {
    /// `menura.integrity_checks`, or its former name.
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
    /// Positional parameters of `sql`, in position order (CHK-7).
    pub parameters: Vec<CheckParameter>,
    /// Catalog relations it reads.
    pub sources: Vec<String>,
    /// The summary's build query: present on every `summary` check and on no other kind
    /// (CHK-5.4); absent before contract version 1.2.0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<BuildQuery>,
}

/// The build query of a summary (CHK-5.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildQuery {
    /// Read-only query in the dialect.
    pub sql: String,
    /// Positional parameters of `sql`, with the same bindings as the check (CHK-7).
    pub parameters: Vec<CheckParameter>,
    /// Result columns, in order.
    pub columns: Vec<String>,
}

/// A positional parameter of a check or build query (CHK-7.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CheckParameter {
    /// A value the compiler fixed; the pipeline binds it unchanged.
    Fixed(Parameter),
    /// A slot the pipeline fills at run time.
    Bound(BoundParameter),
}

/// A parameter slot the pipeline fills at run time (CHK-7.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundParameter {
    /// Position, from 1 (`$1`).
    pub position: u64,
    /// Logical type of the slot: always `text` (CHK-7.2).
    pub logical_type: TextLogicalType,
    /// What the pipeline binds.
    pub binding: Binding,
}

/// The logical type `{"kind": "text"}`, the only type of a parameter slot (CHK-7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "LogicalType", into = "LogicalType")]
pub struct TextLogicalType;

impl TryFrom<LogicalType> for TextLogicalType {
    type Error = String;

    fn try_from(logical_type: LogicalType) -> Result<Self, Self::Error> {
        match logical_type {
            LogicalType::Text => Ok(Self),
            other => Err(format!("a parameter slot has type text, not {other:?}")),
        }
    }
}

impl From<TextLogicalType> for LogicalType {
    fn from(_: TextLogicalType) -> Self {
        LogicalType::Text
    }
}

/// What the pipeline binds to a parameter slot (CHK-7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Binding {
    /// An IANA time-zone name (CHK-7.3).
    TimeZone,
    /// The first period of the range, in canonical form (CHK-7.4).
    PeriodFrom,
    /// The last period of the range, in canonical form (CHK-7.4).
    PeriodTo,
}

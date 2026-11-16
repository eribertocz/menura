//! The catalog schema and catalog state documents (CAT-2 to CAT-6).

use serde::{Deserialize, Serialize};

use super::common::{CheckStatus, Grain, LogicalType, Token};
use crate::contract::ContractVersion;
use crate::hash::Sha256Hash;

/// The `contract` value `semantic_compiler.catalog_schema`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CatalogSchemaContract {
    /// The only value.
    #[serde(rename = "semantic_compiler.catalog_schema")]
    CatalogSchema,
}

/// The catalog schema document: relations, columns and types (CAT-4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogSchemaDocument {
    /// Always `semantic_compiler.catalog_schema`.
    pub contract: CatalogSchemaContract,
    /// Contract version.
    pub contract_version: ContractVersion,
    /// Identifier of this document version (CAT-2.1).
    pub document_id: Token,
    /// Engine, informative only.
    pub engine: Engine,
    /// Relations described.
    pub relations: Vec<RelationSchema>,
}

/// The database engine, never interpreted (CAT-4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Engine {
    /// Engine name.
    pub name: String,
    /// Engine version.
    pub version: String,
}

/// One relation of the catalog schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationSchema {
    /// Identical to the `source` of the Apache Ossie dataset it describes.
    pub source: String,
    /// Columns, at least one.
    pub columns: Vec<Column>,
}

/// One column of a relation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Column {
    /// Column name, as the catalog spells it.
    pub name: String,
    /// Neutral type, the only one the compiler reasons with.
    pub logical_type: LogicalType,
    /// Engine type, kept for lineage only.
    pub physical_type: String,
    /// Whether the column admits NULL.
    pub nullable: bool,
}

/// The `contract` value `semantic_compiler.catalog_state`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CatalogStateContract {
    /// The only value.
    #[serde(rename = "semantic_compiler.catalog_state")]
    CatalogState,
}

/// The catalog state document: data versions, summary builds and check results (CAT-5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogStateDocument {
    /// Always `semantic_compiler.catalog_state`.
    pub contract: CatalogStateContract,
    /// Contract version.
    pub contract_version: ContractVersion,
    /// Identifier of this document version.
    pub document_id: Token,
    /// The schema document this state is consistent with (CAT-2.2).
    pub schema_document_id: Token,
    /// State of each relation.
    pub relations: Vec<RelationState>,
    /// Results of integrity checks other than summary checks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checks: Option<Vec<CheckResult>>,
}

/// The state of one relation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RelationStateMembers", into = "RelationStateMembers")]
pub struct RelationState {
    /// The relation, as in the schema document.
    pub source: String,
    /// Estimate used only to choose among valid summaries.
    pub row_count_estimate: u64,
    /// Exactly one of `version`, `partitions` and `aggregate_build`.
    pub data: RelationData,
}

/// How the data of a relation is versioned (CAT-5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationData {
    /// A relation that is not partitioned: the version of the whole relation.
    Version(Token),
    /// A partitioned base relation: the version of each period.
    Partitions(Partitions),
    /// A summary: how it was built.
    AggregateBuild(AggregateBuild),
}

/// The data version of each period of a partitioned relation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Partitions {
    /// Partitioning column.
    pub column: String,
    /// Grain of the periods.
    pub grain: Grain,
    /// Every period that contains rows (CAT-7.1.5).
    pub versions: Vec<PeriodVersion>,
}

/// The version of one canonical period (CAT-5.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeriodVersion {
    /// Canonical period.
    pub period: String,
    /// Data version of the period.
    pub version: Token,
}

/// How a summary was built.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AggregateBuild {
    /// Hash of the definition the summary was built from.
    pub definition_hash: Sha256Hash,
    /// Periods covered.
    pub coverage: Coverage,
    /// Base version each period was built from.
    pub built_from: Vec<PeriodVersion>,
    /// Last integrity check of the summary.
    pub integrity_check: SummaryCheck,
    /// IANA time zone used to truncate periods.
    pub time_zone: String,
}

/// An inclusive range of periods.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    /// First period.
    pub from: String,
    /// Last period.
    pub to: String,
}

/// The result of a summary's integrity check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SummaryCheck {
    /// Result.
    pub status: CheckStatus,
    /// Hash of the check definition that was run.
    pub definition_hash: Sha256Hash,
    /// Base versions the check was run on.
    pub verified: Vec<PeriodVersion>,
}

/// Kinds of check reported in `checks` (summary checks are reported in `aggregate_build`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateCheckKind {
    /// Relationship key uniqueness.
    KeyUniqueness,
    /// Foreign keys without a match.
    OrphanKeys,
    /// Population extension.
    Population,
    /// Semi-additive metric extension.
    SemiAdditive,
}

/// The result of one integrity check (CAT-7.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckResult {
    /// Relationship, dataset or metric checked.
    pub target: String,
    /// Kind of check.
    pub check: StateCheckKind,
    /// Hash of the check definition that was run.
    pub definition_hash: Sha256Hash,
    /// Result.
    pub status: CheckStatus,
    /// Data versions the check was run on.
    pub verified: Vec<VerifiedVersion>,
}

/// One data version a check was run on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifiedVersion {
    /// Relation.
    pub source: String,
    /// Data version.
    pub version: Token,
    /// Period, absent for a relation that is not partitioned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period: Option<String>,
}

/// The members of a relation state as they appear in JSON, before checking that exactly one of
/// the three data members is present.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RelationStateMembers {
    source: String,
    row_count_estimate: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    version: Option<Token>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    partitions: Option<Partitions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    aggregate_build: Option<AggregateBuild>,
}

impl TryFrom<RelationStateMembers> for RelationState {
    type Error = &'static str;

    fn try_from(members: RelationStateMembers) -> Result<Self, Self::Error> {
        let data = match (members.version, members.partitions, members.aggregate_build) {
            (Some(version), None, None) => RelationData::Version(version),
            (None, Some(partitions), None) => RelationData::Partitions(partitions),
            (None, None, Some(build)) => RelationData::AggregateBuild(build),
            _ => {
                return Err("a relation needs exactly one of its three data members");
            }
        };
        Ok(Self {
            source: members.source,
            row_count_estimate: members.row_count_estimate,
            data,
        })
    }
}

impl From<RelationState> for RelationStateMembers {
    fn from(state: RelationState) -> Self {
        let (version, partitions, aggregate_build) = match state.data {
            RelationData::Version(version) => (Some(version), None, None),
            RelationData::Partitions(partitions) => (None, Some(partitions), None),
            RelationData::AggregateBuild(build) => (None, None, Some(build)),
        };
        Self {
            source: state.source,
            row_count_estimate: state.row_count_estimate,
            version,
            partitions,
            aggregate_build,
        }
    }
}

//! The query document (QRY-2 to QRY-4).

use serde::{Deserialize, Serialize};

use super::common::Grain;
use crate::contract::ContractVersion;

/// The `contract` value `semantic_compiler.query`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryContract {
    /// The only value.
    #[serde(rename = "semantic_compiler.query")]
    Query,
}

/// A query document: the envelope and exactly one form (QRY-2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "QueryMembers", into = "QueryMembers")]
pub struct QueryDocument {
    /// Always `semantic_compiler.query`.
    pub contract: QueryContract,
    /// Contract version.
    pub contract_version: ContractVersion,
    /// RFC 3339 instant with offset, present when the query needs "now".
    pub reference_time: Option<String>,
    /// IANA time zone name.
    pub time_zone: String,
    /// The question, in one of its two forms.
    pub form: QueryForm,
}

/// The two forms of a query (QRY-1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryForm {
    /// Wide-table form, the Apache Ossie Layer 3 JSON binding (`query`).
    WideTable(WideTableQuery),
    /// SQL form, Apache Ossie Layer 2 (`sql`).
    Sql(String),
}

/// The wide-table form (QRY-3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WideTableQuery {
    /// Dimensions, in output order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<Vec<DimensionItem>>,
    /// Metrics, in output order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics: Option<Vec<MetricItem>>,
    /// Filter expression.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    /// At most one compiler extension entry (QRY-4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nonportable_extensions: Option<Vec<Extension>>,
}

/// A requested dimension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DimensionItem {
    /// A bare `dataset.field` reference.
    Reference(String),
    /// A reference with an optional output alias.
    Aliased(AliasedField),
}

/// A dimension written as an object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AliasedField {
    /// `dataset.field`.
    pub field: String,
    /// Output name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}

/// A requested metric.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetricItem {
    /// A bare metric reference (QRY-3.2).
    Reference(String),
    /// A reference with an optional output alias.
    Aliased(AliasedMetric),
}

/// A metric written as an object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AliasedMetric {
    /// Metric reference.
    pub metric: String,
    /// Output name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}

/// The `vendor_name` value `COMMON`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VendorName {
    /// The only value.
    #[serde(rename = "COMMON")]
    Common,
}

/// The `namespace` value `semantic_compiler`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Namespace {
    /// The only value.
    #[serde(rename = "semantic_compiler")]
    SemanticCompiler,
}

/// One `nonportable_extensions` entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Extension {
    /// Always `COMMON`.
    pub vendor_name: VendorName,
    /// The compiler's extension data.
    pub data: CompilerExtension,
}

/// The compiler's query extensions (QRY-4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompilerExtension {
    /// Always `semantic_compiler`.
    pub namespace: Namespace,
    /// Extension version.
    pub version: u64,
    /// QRY-4.1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_grains: Option<Vec<TimeGrain>>,
    /// QRY-4.2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_range: Option<DateRange>,
    /// QRY-4.3.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paths: Option<Vec<PathChoice>>,
    /// QRY-4.4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<String>>,
    /// QRY-4.5.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_by: Option<Vec<OrderItem>>,
    /// QRY-4.6.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
}

/// A time grain applied to a dimension (QRY-4.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeGrain {
    /// Dimension reference.
    pub dimension: String,
    /// Grain.
    pub grain: Grain,
}

/// A date range on a dimension (QRY-4.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "DateRangeMembers", into = "DateRangeMembers")]
pub struct DateRange {
    /// Dimension reference.
    pub dimension: String,
    /// Absolute or relative bounds.
    pub bounds: DateBounds,
}

/// The bounds of a date range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateBounds {
    /// Inclusive dates, `YYYY-MM-DD`.
    Absolute {
        /// First date.
        from: String,
        /// Last date.
        to: String,
    },
    /// Whole units relative to `reference_time`.
    Relative(RelativeRange),
}

/// A relative date range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelativeRange {
    /// Unit.
    pub unit: Grain,
    /// Number of units, at least 1.
    pub count: u64,
    /// Whether the current unit is included; `true` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_current: Option<bool>,
}

/// A path choice between two datasets (QRY-4.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathChoice {
    /// Starting dataset.
    pub from: String,
    /// Ending dataset.
    pub to: String,
    /// Relationships, in order, at least one.
    pub relationships: Vec<String>,
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Ascending.
    Asc,
    /// Descending.
    Desc,
}

/// Placement of NULLs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NullsOrder {
    /// NULLs first.
    First,
    /// NULLs last.
    Last,
}

/// One ordering term (QRY-4.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderItem {
    /// Output name.
    pub output: String,
    /// Direction.
    pub direction: Direction,
    /// NULL placement.
    pub nulls: NullsOrder,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct QueryMembers {
    contract: QueryContract,
    contract_version: ContractVersion,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reference_time: Option<String>,
    time_zone: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    query: Option<WideTableQuery>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sql: Option<String>,
}

impl TryFrom<QueryMembers> for QueryDocument {
    type Error = &'static str;

    fn try_from(members: QueryMembers) -> Result<Self, Self::Error> {
        let form = match (members.query, members.sql) {
            (Some(query), None) => QueryForm::WideTable(query),
            (None, Some(sql)) => QueryForm::Sql(sql),
            _ => return Err("a query has exactly one of query and sql"),
        };
        Ok(Self {
            contract: members.contract,
            contract_version: members.contract_version,
            reference_time: members.reference_time,
            time_zone: members.time_zone,
            form,
        })
    }
}

impl From<QueryDocument> for QueryMembers {
    fn from(document: QueryDocument) -> Self {
        let (query, sql) = match document.form {
            QueryForm::WideTable(query) => (Some(query), None),
            QueryForm::Sql(sql) => (None, Some(sql)),
        };
        Self {
            contract: document.contract,
            contract_version: document.contract_version,
            reference_time: document.reference_time,
            time_zone: document.time_zone,
            query,
            sql,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DateRangeMembers {
    dimension: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    relative: Option<RelativeRange>,
}

impl TryFrom<DateRangeMembers> for DateRange {
    type Error = &'static str;

    fn try_from(members: DateRangeMembers) -> Result<Self, Self::Error> {
        let bounds = match (members.from, members.to, members.relative) {
            (Some(from), Some(to), None) => DateBounds::Absolute { from, to },
            (None, None, Some(relative)) => DateBounds::Relative(relative),
            _ => return Err("a date range has either from and to, or relative"),
        };
        Ok(Self {
            dimension: members.dimension,
            bounds,
        })
    }
}

impl From<DateRange> for DateRangeMembers {
    fn from(range: DateRange) -> Self {
        let (from, to, relative) = match range.bounds {
            DateBounds::Absolute { from, to } => (Some(from), Some(to), None),
            DateBounds::Relative(relative) => (None, None, Some(relative)),
        };
        Self {
            dimension: range.dimension,
            from,
            to,
            relative,
        }
    }
}

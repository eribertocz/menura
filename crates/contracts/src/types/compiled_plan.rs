//! The compiled plan and compile error documents (PLN-1, PLN-5).

use serde::{Deserialize, Serialize};

use super::common::{Decision, Dialect, LogicalType, Parameter, Producer, Token};
use crate::contract::ContractVersion;
use crate::hash::Sha256Hash;

/// The `contract` value of a compiled plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledPlanContract {
    /// `menura.compiled_plan`: an executable plan.
    #[serde(rename = "menura.compiled_plan")]
    CompiledPlan,
    /// `menura.explanation`: the same document without SQL or parameters.
    #[serde(rename = "menura.explanation")]
    Explanation,
    /// `semantic_compiler.compiled_plan`: the former name, read until 1.0.0 and never written
    /// (CON-0.5).
    #[serde(rename = "semantic_compiler.compiled_plan")]
    FormerCompiledPlan,
    /// `semantic_compiler.explanation`: the former name, read until 1.0.0 and never written
    /// (CON-0.5).
    #[serde(rename = "semantic_compiler.explanation")]
    FormerExplanation,
}

/// A compiled plan (PLN-1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledPlanDocument {
    /// `menura.compiled_plan` or `menura.explanation`, or a former name (CON-0.5).
    pub contract: CompiledPlanContract,
    /// Contract version.
    pub contract_version: ContractVersion,
    /// Compiler and specification versions.
    pub producer: Producer,
    /// Target dialect.
    pub dialect: Dialect,
    /// Logical source name.
    pub data_source: String,
    /// One SQL statement.
    pub sql: String,
    /// Bound parameters, by position.
    pub parameters: Vec<Parameter>,
    /// Output columns, in the query tree's column order.
    pub result_schema: Vec<ResultColumn>,
    /// Cache key (PLN-3).
    pub cache_key: CacheKey,
    /// Suggested limits.
    pub limits: Limits,
    /// One entry per output column (PLN-4).
    pub lineage: Vec<Lineage>,
    /// Every decision with its stage and reason.
    pub explanation: Vec<Decision>,
    /// Non-blocking diagnostics.
    pub warnings: Vec<Diagnostic>,
}

/// Role of an output column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnRole {
    /// A grouping column.
    Dimension,
    /// An aggregated column.
    Metric,
}

/// One output column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultColumn {
    /// Output name.
    pub name: String,
    /// Dimension or metric.
    pub role: ColumnRole,
    /// Logical type.
    pub logical_type: LogicalType,
    /// Engine type.
    pub physical_type: String,
    /// Whether the column may hold NULL.
    pub nullable: bool,
}

/// The cache key and its parts (PLN-3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CacheKey {
    /// Hash of `{plan_hash, security_fingerprint, data_versions}`.
    pub key: Sha256Hash,
    /// Hash of `{dialect, data_source, sql, parameters, result_schema}`.
    pub plan_hash: Sha256Hash,
    /// Security context fingerprint (SEC-3.6).
    pub security_fingerprint: Sha256Hash,
    /// Versions read, or `null` when some relation read has none.
    pub data_versions: Option<Vec<DataVersion>>,
}

/// The version of one period read (`*` for a relation that is not partitioned).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataVersion {
    /// Relation.
    pub source: String,
    /// Period, or `*`.
    pub period: String,
    /// Data version.
    pub version: Token,
}

/// Suggested execution limits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    /// Suggested timeout in milliseconds.
    pub timeout_ms: u64,
    /// Suggested maximum rows; `null` means no limit beyond the query's own.
    pub max_rows: Option<u64>,
}

/// Lineage of one output column (PLN-4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lineage {
    /// Output column.
    pub output: String,
    /// Model fields and metrics it derives from.
    pub model_inputs: Vec<String>,
    /// Catalog relations and columns read.
    pub physical_inputs: Vec<PhysicalInput>,
    /// Summary dataset used, or `null`.
    pub summary: Option<String>,
    /// Row policies applied to the rows behind the column.
    pub security_policies: Vec<String>,
}

/// One catalog column read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalInput {
    /// Relation.
    pub source: String,
    /// Column.
    pub column: String,
}

/// The `contract` value of a compile error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompileErrorContract {
    /// `menura.compile_error`: at least one error.
    #[serde(rename = "menura.compile_error")]
    CompileError,
    /// `menura.validation_report`: the diagnostics of a validation, possibly none.
    #[serde(rename = "menura.validation_report")]
    ValidationReport,
    /// `semantic_compiler.compile_error`: the former name, read until 1.0.0 and never written
    /// (CON-0.5).
    #[serde(rename = "semantic_compiler.compile_error")]
    FormerCompileError,
    /// `semantic_compiler.validation_report`: the former name, read until 1.0.0 and never
    /// written (CON-0.5).
    #[serde(rename = "semantic_compiler.validation_report")]
    FormerValidationReport,
}

/// A compile error (PLN-5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompileErrorDocument {
    /// `menura.compile_error` or `menura.validation_report`, or a former name (CON-0.5).
    pub contract: CompileErrorContract,
    /// Contract version.
    pub contract_version: ContractVersion,
    /// Errors; the first follows the precedence of E2-5 or E0-4.3.
    pub errors: Vec<Diagnostic>,
    /// Diagnostics gathered before the failure.
    pub warnings: Vec<Diagnostic>,
}

/// One diagnostic (PLN-5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    /// Specification code.
    pub code: String,
    /// English message naming the offending item and the fix.
    pub message: String,
    /// Where the problem is, or `null`.
    pub location: Option<DiagnosticLocation>,
    /// For `E_NAME_NOT_FOUND`, the names the user may write instead (PLN-5.4); absent when
    /// there are none. Boxed, because it is rare and diagnostics travel in `Err` values.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alternatives: Option<Box<Alternatives>>,
}

/// The names a user may write instead of an unknown one (PLN-5.4), each `field:`, `metric:` or
/// `dataset:` followed by the name, at most five.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Alternatives(pub Vec<String>);

/// An input document of `load` or `compile`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputDocument {
    /// The Apache Ossie model.
    Model,
    /// The catalog schema.
    CatalogSchema,
    /// The catalog state.
    CatalogState,
    /// The query.
    Query,
    /// The security context.
    SecurityContext,
}

/// The location of a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticLocation {
    /// Input document.
    pub document: InputDocument,
    /// JSON Pointer (RFC 6901) inside the document.
    pub pointer: String,
    /// Line, from 1, for YAML or SQL text when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u64>,
    /// Column, from 1, for YAML or SQL text when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<u64>,
}

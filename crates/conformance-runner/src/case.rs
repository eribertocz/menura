//! The case format of `spec/08-conformance-suite.md` CS-2, as data types only.
//!
//! Reading and validating a case file is done by [`crate::case_file`].

use std::fmt;

use menura_contracts::Sha256Hash;
use menura_contracts::types::audit_record::OssieFormat;
use menura_contracts::types::compiled_plan::InputDocument;
use serde::Deserialize;

/// A rule ID of the specification, such as `E3-3.2` or `CS-2.1`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(transparent)]
pub struct RuleId(String);

impl RuleId {
    /// A rule ID from its text, for cases the runner builds itself (the Apache Ossie suites).
    #[must_use]
    pub fn new(rule: &str) -> Self {
        Self(rule.to_owned())
    }

    /// The rule ID as written in the specification.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RuleId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The identifier of a case, such as `e3-ambiguous-path-001`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(transparent)]
pub struct CaseId(String);

impl CaseId {
    /// A case identifier from its text, for cases the runner builds itself (the Apache Ossie
    /// suites).
    #[must_use]
    pub fn new(id: String) -> Self {
        Self(id)
    }

    /// The case identifier as written in the case file.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CaseId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// One conformance case (CS-2).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    /// The case identifier; it matches the file name without extension.
    pub id: CaseId,
    /// Every rule ID this case covers (CS-2.1: at least one).
    pub rules: Vec<RuleId>,
    /// The folder under `conformance/models/` holding the model and its fixtures.
    pub model: String,
    /// The security context file under `models/<model>/contexts/`.
    pub security_context: Option<String>,
    /// The query document, or `None` for load-only cases.
    pub query: Option<serde_json::Value>,
    /// The load options the case loads its model with, or `None` for all defaults (CS-2.7).
    pub load_options: Option<CaseLoadOptions>,
    /// The compile options the case sets, or `None` for all defaults (CS-2.18).
    pub compile_options: Option<CaseCompileOptions>,
    /// The oracle of a differential case: a reference SQL file next to the case, whose rows the
    /// rows of the plan are also compared with (CS-4.8). Its presence makes the case
    /// differential.
    pub oracle: Option<String>,
    /// The expected result.
    pub expect: Expectation,
}

/// The load options a case sets (CS-2.7), as written: an omitted option is `None` and takes the
/// default of architecture section 3.5 (see [`crate::load_options::resolve_load_options`]).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseLoadOptions {
    /// `release` or `development`.
    pub ossie_format: Option<OssieFormat>,
    /// The semantic model to load when the document has several (DEV-7).
    pub model_name: Option<String>,
    /// The extension namespace (MDL-2.3).
    pub namespace: Option<String>,
    /// Treat datasets without `access` as `authenticated` (DEV-4).
    pub grant_authenticated_to_all: Option<bool>,
}

/// The compile options a case sets (CS-2.18): only `actor`. Every other compile option keeps its
/// default, except `audit` (CS-2.13, CS-2.16).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseCompileOptions {
    /// The delegated actor, a string, passed to `compile` and `explain` (PLN-6.8). Any other
    /// value makes the case file malformed.
    pub actor: Option<String>,
}

/// The expected result of a case (CS-2 `expect`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expectation {
    /// The kind of result.
    pub outcome: Outcome,
    /// The first error code (E2-5.2), or the runtime code for `runtime_error` (CS-2.5).
    pub error: Option<String>,
    /// The `alternatives` of the first diagnostic, in order; an empty list asserts that the
    /// diagnostic has none (CS-2.17).
    pub alternatives: Option<Vec<String>>,
    /// Load warnings that must be reported, by code.
    #[serde(default)]
    pub warnings: Vec<String>,
    /// The location of the first error, compared as CS-2.9 says.
    pub location: Option<ExpectedLocation>,
    /// Golden SQL file next to the case, compared byte for byte (CS-2.3).
    pub sql: Option<String>,
    /// Golden `ClickHouse` SQL file next to the case, compared like `sql` (CS-2.3a).
    pub sql_clickhouse: Option<String>,
    /// What compiling with `dialect: clickhouse` returns instead of golden `ClickHouse` SQL
    /// (CS-2.3a).
    pub clickhouse: Option<ClickhouseExpectation>,
    /// Expected rows file next to the case, compared as CS-2.4 says.
    pub rows: Option<String>,
    /// How the order of `rows` is compared.
    pub row_order: Option<RowOrder>,
    /// Relative tolerance for float columns.
    pub tolerance: Option<f64>,
    /// Golden JSON file of the plan's `lineage` (CS-2.11).
    pub lineage: Option<String>,
    /// Golden JSON file of the plan's `data_versions`, a JSON array or `null` (CS-2.11).
    pub data_versions: Option<String>,
    /// The `code` of every entry of the plan's `explanation`, in order (CS-2.12).
    pub explanation: Option<Vec<String>>,
    /// Golden JSON file of the audit record, without its `producer` (CS-2.13).
    pub audit: Option<String>,
    /// The plan's `cache_key.key` (CS-2.14).
    pub cache_key: Option<Sha256Hash>,
    /// Golden JSON file of the `describe` or `integrity_checks` output document (CS-2.15).
    pub document: Option<String>,
}

/// The expected location of the first error (CS-2 `expect.location`, CS-2.9).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedLocation {
    /// The input document, compared exactly.
    pub document: InputDocument,
    /// The JSON Pointer inside the document, compared exactly.
    pub pointer: String,
    /// The line, from 1, compared only when the case gives it.
    pub line: Option<u64>,
    /// The column, from 1, compared only when the case gives it.
    pub column: Option<u64>,
}

/// The kind of result a case expects (CS-2 `expect.outcome`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Loading the model fails.
    LoadError,
    /// Compiling the query fails.
    Error,
    /// Compiling succeeds and the plan is executed.
    Plan,
    /// The generated SQL fails at execution with a runtime code.
    RuntimeError,
    /// `describe` returns the model description of the security context (CS-2.15).
    Describe,
    /// `integrity_checks` returns the integrity checks for the dialect `duckdb` (CS-2.15).
    IntegrityChecks,
}

impl Outcome {
    /// Whether the case must name a security context (CS-2, CS-2.15).
    #[must_use]
    pub fn needs_security_context(self) -> bool {
        !matches!(self, Self::LoadError | Self::IntegrityChecks)
    }

    /// Whether the outcome returns an output document compared with `document` (CS-2.15).
    #[must_use]
    pub fn returns_document(self) -> bool {
        matches!(self, Self::Describe | Self::IntegrityChecks)
    }
}

/// What a case expects when compiled with `dialect: clickhouse`, when it has no golden
/// `ClickHouse` SQL (CS-2 `expect.clickhouse`, CS-2.3a).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ClickhouseExpectation {
    /// The query needs a value `ClickHouse` cannot express, so compilation returns `E6-01`
    /// (E6-7.12).
    #[serde(rename = "e6_01")]
    Inexpressible,
}

/// How the order of expected rows is compared (CS-2 `expect.row_order`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowOrder {
    /// Rows match as a multiset.
    Any,
    /// Rows match in the same order.
    Exact,
}

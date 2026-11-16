//! The case format of `spec/08-conformance-suite.md` CS-2, as data types only.
//!
//! Reading and validating a case file is done by [`crate::case_file`].

use std::fmt;

use serde::Deserialize;

/// A rule ID of the specification, such as `E3-3.2` or `CS-2.1`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(transparent)]
pub struct RuleId(String);

impl RuleId {
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
    /// The expected result.
    pub expect: Expectation,
}

/// The expected result of a case (CS-2 `expect`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expectation {
    /// The kind of result.
    pub outcome: Outcome,
    /// The first error code (E2-5.2), or the runtime code for `runtime_error` (CS-2.5).
    pub error: Option<String>,
    /// Load warnings that must be reported, by code.
    #[serde(default)]
    pub warnings: Vec<String>,
    /// Golden SQL file next to the case, compared byte for byte (CS-2.3).
    pub sql: Option<String>,
    /// Expected rows file next to the case, compared as CS-2.4 says.
    pub rows: Option<String>,
    /// How the order of `rows` is compared.
    pub row_order: Option<RowOrder>,
    /// Relative tolerance for float columns.
    pub tolerance: Option<f64>,
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
}

impl Outcome {
    /// Whether the case must name a security context (CS-2).
    #[must_use]
    pub fn needs_security_context(self) -> bool {
        !matches!(self, Self::LoadError)
    }
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

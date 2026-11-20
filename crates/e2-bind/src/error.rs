//! The errors of E2 (E2-6), each with its specification code and its location in the query.

use std::fmt;

use menura_contracts::types::compiled_plan::{Diagnostic, DiagnosticLocation, InputDocument};
use menura_representations::origin::QueryLocation;

/// The codes rows 3 to 9 of E2-5 report.
///
/// Row 7, `E_WINDOW_IN_WHERE`, has no code here: the expression language rejects a window
/// function as `E0F-02` (E0F-2.4), which E1 reports at row 1, so no query tree holds one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BindCode {
    /// `E_NAME_NOT_FOUND`: an unknown or unauthorized name (row 3).
    NameNotFound,
    /// `E_METRIC_IN_DIMENSIONS`: a metric used as a dimension (row 4).
    MetricInDimensions,
    /// `E_FIELD_IN_METRICS`: a field used as a metric (row 5).
    FieldInMetrics,
    /// `E_AGGREGATE_IN_WHERE`: an aggregate or a metric in a filter (row 6).
    AggregateInWhere,
    /// `E_CROSS_DATASET_FILTER`: a filter that reads fields of more than one dataset (row 8).
    CrossDatasetFilter,
    /// `E_NAME_COLLISION`: a repeated output name (row 9).
    NameCollision,
}

impl BindCode {
    /// The code as the specification writes it.
    ///
    /// ```
    /// use menura_e2_bind::BindCode;
    /// assert_eq!(BindCode::NameNotFound.as_str(), "E_NAME_NOT_FOUND");
    /// ```
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NameNotFound => "E_NAME_NOT_FOUND",
            Self::MetricInDimensions => "E_METRIC_IN_DIMENSIONS",
            Self::FieldInMetrics => "E_FIELD_IN_METRICS",
            Self::AggregateInWhere => "E_AGGREGATE_IN_WHERE",
            Self::CrossDatasetFilter => "E_CROSS_DATASET_FILTER",
            Self::NameCollision => "E_NAME_COLLISION",
        }
    }
}

impl fmt::Display for BindCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A diagnostic of E2: its code, its English message and the element of the query it is
/// located at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindDiagnostic {
    /// The code.
    pub code: BindCode,
    /// The message: it names the item as written and the fix, and never reveals what the user
    /// may not see (PLN-5.2).
    pub message: String,
    /// The element of the query document it is located at.
    pub location: QueryLocation,
}

impl BindDiagnostic {
    /// The diagnostic `code` at `location`, with `message`.
    pub(crate) fn new(code: BindCode, message: String, location: &QueryLocation) -> Self {
        Self {
            code,
            message,
            location: location.clone(),
        }
    }

    /// The diagnostic as the compile error document writes it (PLN-5), in the query document.
    #[must_use]
    pub fn to_diagnostic(&self) -> Diagnostic {
        let position = self.location.position();
        Diagnostic {
            code: self.code.as_str().to_owned(),
            message: self.message.clone(),
            location: Some(DiagnosticLocation {
                document: InputDocument::Query,
                pointer: self.location.pointer().to_owned(),
                line: position.and_then(|position| u64::try_from(position.line()).ok()),
                column: position.and_then(|position| u64::try_from(position.column()).ok()),
            }),
        }
    }
}

/// Why E2 stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindError {
    /// The first error of the query, in the order of E2-5 (E2-5.2).
    Diagnostic(BindDiagnostic),
    /// The query tree breaks the contract of E1 at an element. This is a defect of the stage
    /// that built the tree, not a query error, so it has no specification code.
    Invariant {
        /// What the tree breaks.
        violation: InvariantViolation,
        /// The element.
        location: QueryLocation,
    },
}

/// How a query tree breaks the contract of E1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvariantViolation {
    /// An element of the SQL form: an expression dimension or an inline metric. E1 stops the
    /// SQL form before 0.5.0 (E1-2.2, VS-2).
    SqlFormElement,
    /// A path without relationships (QRY-4.3).
    PathWithoutRelationship,
}

impl BindError {
    /// The invariant error `violation` at `location`.
    pub(crate) fn invariant(violation: InvariantViolation, location: &QueryLocation) -> Self {
        Self::Invariant {
            violation,
            location: location.clone(),
        }
    }
}

impl From<BindDiagnostic> for BindError {
    fn from(diagnostic: BindDiagnostic) -> Self {
        Self::Diagnostic(diagnostic)
    }
}

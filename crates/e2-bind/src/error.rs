//! The errors of E2 (E2-6), each with its specification code and its location in the query.

use std::fmt;

use menura_contracts::types::compiled_plan::{Diagnostic, DiagnosticLocation, InputDocument};
use menura_expressions::typing::TypingCode;
use menura_model::internal_model::DiagnosticCode;
use menura_representations::origin::QueryLocation;

/// The codes rows 3 to 12c of E2-5 report.
///
/// Row 7, `E_WINDOW_IN_WHERE`, has no code here: the expression language rejects a window
/// function as `E0F-02` (E0F-2.4), which E1 reports at row 1, so no query tree holds one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    /// `E_UNAGGREGATED_FINER_GRAIN_REFERENCE`: a metric made unusable by that rule (row 10).
    UnaggregatedFinerGrainReference,
    /// `E_DEFERRED_WINDOWED_METRIC`: a metric made unusable by a window function (row 11).
    DeferredWindowedMetric,
    /// `E_AMBIGUOUS_MEASURE_GRAIN`: a metric made unusable by that rule (row 12).
    AmbiguousMeasureGrain,
    /// The query error code of an element marked unusable in E0, such as `E0D-03` (row 12a,
    /// E0-4).
    Unusable(DiagnosticCode),
    /// The E0.f code of a typing problem in a filter, `E0F-03` to `E0F-08`; `E0F-06` for a
    /// filter that is not boolean (row 12b, E2-3.6).
    FilterType(TypingCode),
    /// `E2-01`: a time grain or a date range on a field that is not a time field (E2-3.4).
    NotTimeField,
    /// `EQRY-09`: a time grain on a field that is not a dimension, or a second grain for one
    /// dimension (QRY-4.1).
    TimeGrainNotDimension,
    /// `EQRY-10`: an `order_by` output that names no output (QRY-4.5).
    UnknownOrderOutput,
    /// `EQRY-11`: a date range whose `from` is after its `to` (QRY-4.2).
    ReversedDateRange,
    /// `EQRY-12`: a path whose relationships do not chain, or a second path for one pair
    /// (QRY-4.3).
    InvalidPath,
    /// `EQRY-03`: the query needs `reference_time` and has none: a relative date range, or a
    /// filter that reads the current date or time (QRY-2).
    MissingReferenceTime,
    /// `E_INVALID_QUERY`: a relative date range whose whole units leave the years 0001–9999
    /// (QRY-4.2).
    InvalidQuery,
    /// `EQRY-04`: a `limit` without an `order_by` over every dimension (QRY-4.6).
    LimitWithoutTotalOrder,
    /// `E_NOT_YET_SUPPORTED`: the query needs a capability outside the version's slice
    /// (row 12c, VS-1, VS-2).
    NotYetSupported,
}

impl BindCode {
    /// The code as the specification writes it.
    ///
    /// ```
    /// use menura_e2_bind::BindCode;
    /// assert_eq!(BindCode::NameNotFound.as_str(), "E_NAME_NOT_FOUND");
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::NameNotFound => "E_NAME_NOT_FOUND",
            Self::MetricInDimensions => "E_METRIC_IN_DIMENSIONS",
            Self::FieldInMetrics => "E_FIELD_IN_METRICS",
            Self::AggregateInWhere => "E_AGGREGATE_IN_WHERE",
            Self::CrossDatasetFilter => "E_CROSS_DATASET_FILTER",
            Self::NameCollision => "E_NAME_COLLISION",
            Self::UnaggregatedFinerGrainReference => "E_UNAGGREGATED_FINER_GRAIN_REFERENCE",
            Self::DeferredWindowedMetric => "E_DEFERRED_WINDOWED_METRIC",
            Self::AmbiguousMeasureGrain => "E_AMBIGUOUS_MEASURE_GRAIN",
            Self::Unusable(code) => code.as_str(),
            Self::FilterType(code) => code.as_str(),
            Self::NotTimeField => "E2-01",
            Self::TimeGrainNotDimension => "EQRY-09",
            Self::UnknownOrderOutput => "EQRY-10",
            Self::ReversedDateRange => "EQRY-11",
            Self::InvalidPath => "EQRY-12",
            Self::MissingReferenceTime => "EQRY-03",
            Self::InvalidQuery => "E_INVALID_QUERY",
            Self::LimitWithoutTotalOrder => "EQRY-04",
            Self::NotYetSupported => "E_NOT_YET_SUPPORTED",
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
    /// A date range whose bounds cannot be resolved to dates: a bound that is not a date,
    /// or a target that rows 3 to 12b left without a time field or a reference time.
    UnresolvableDateRange,
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

//! The diagnostics of metric classification (E0G-2, E0G-7), as data types only.

use std::fmt;

use super::output::QueryError;
use crate::pointer::JsonPointer;
use crate::read::Location;

/// The query error code of a metric an E0G-2 rule or an additivity conflict (E0G-4.1, E0G-4.2)
/// makes unusable. Every one is reported as `W0G-01` (E0-4 table).
///
/// The order of the variants is the precedence of E2-5 (rows 10, 12 and 12a), so the lowest
/// code among the rules a metric breaks is the one recorded (E0G-2.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClassificationCode {
    /// `E_UNAGGREGATED_FINER_GRAIN_REFERENCE`: a field reference outside every aggregate
    /// (E0G-2.1).
    UnaggregatedFinerGrainReference,
    /// `E_AMBIGUOUS_MEASURE_GRAIN`: aggregate arguments that read more than one dataset
    /// (E0G-2.3), a `FILTER` that reads another dataset (E0G-2.4), or, in a model-level metric,
    /// aggregate arguments that read no dataset (E0G-2.5).
    AmbiguousMeasureGrain,
    /// `E0G-01`: an additivity declaration that relaxes the deduced additivity (E0G-4.1), or a
    /// semi-additive metric whose components are on more than one home dataset (E0G-4.2).
    AdditivityConflict,
    /// `E0G-02`: a base metric with no aggregate (E0G-2.7).
    NoAggregate,
}

impl ClassificationCode {
    /// The load warning reported for every metric classification problem (E0-4 table).
    pub const WARNING: &'static str = "W0G-01";

    /// The code as the specification writes it, such as `E0G-02`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnaggregatedFinerGrainReference => "E_UNAGGREGATED_FINER_GRAIN_REFERENCE",
            Self::AmbiguousMeasureGrain => "E_AMBIGUOUS_MEASURE_GRAIN",
            Self::AdditivityConflict => "E0G-01",
            Self::NoAggregate => "E0G-02",
        }
    }
}

impl fmt::Display for ClassificationCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One classification diagnostic: its code, the metric it is about and a description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassificationDiagnostic {
    /// The query error code of the metric (E0G-2.8, DER-5.1.5).
    pub code: QueryError,
    /// The JSON Pointer of the metric.
    pub pointer: JsonPointer,
    /// Where the metric starts in the document text.
    pub location: Location,
    /// What the problem is, in English, for diagnostics.
    pub message: String,
}

impl fmt::Display for ClassificationDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} ({}) at `{}` (line {}, column {}): {}",
            ClassificationCode::WARNING,
            self.code.as_str(),
            self.pointer,
            self.location.line,
            self.location.column,
            self.message
        )
    }
}

impl std::error::Error for ClassificationDiagnostic {}

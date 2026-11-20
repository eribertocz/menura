//! The invariant violations of stage E3 in the 0.3.0 slice.

use std::fmt;

use menura_model::internal_model::NormalizedName;
use menura_representations::origin::QueryLocation;

/// A bound query that E2 cannot produce in the 0.3.0 slice (VS-1, E2-5 row 12c).
///
/// This is an internal error of the compiler, not a specification diagnostic: no query that
/// passes E2 meets any of these conditions, so none has a specification code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvariantViolation {
    /// The query has neither metrics nor dimensions.
    NoMetricAndNoDimension,
    /// The query chooses a relationship path; the slice has no `paths` (VS-1).
    ChosenPath {
        /// Where the first path is written.
        origin: QueryLocation,
    },
    /// An inline metric, which only the SQL form writes (0.5.0).
    InlineMetric {
        /// Where the metric is written.
        origin: QueryLocation,
    },
    /// A metric that the model does not have, or that E0.g did not classify (E2-5 row 12a).
    UnclassifiedMetric {
        /// Where the metric is written.
        origin: QueryLocation,
    },
    /// A metric without components, such as a derived metric (0.4.0).
    MetricWithoutComponents {
        /// Where the metric is written.
        origin: QueryLocation,
    },
    /// The query reads more than one dataset.
    SeveralDatasets {
        /// The first dataset the query reads, in query order (E2-5.3).
        first: NormalizedName,
        /// The first other dataset, in query order.
        second: NormalizedName,
        /// Where the element that reads `second` is written.
        origin: QueryLocation,
    },
    /// The branches were rejected because two of them share a home dataset (E3-4.1).
    DuplicateHomeDataset,
}

impl fmt::Display for InvariantViolation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoMetricAndNoDimension => {
                formatter.write_str("the bound query has neither metrics nor dimensions")
            }
            Self::ChosenPath { origin } => {
                write!(
                    formatter,
                    "the bound query chooses a path at {}",
                    origin.pointer()
                )
            }
            Self::InlineMetric { origin } => {
                write!(formatter, "inline metric at {}", origin.pointer())
            }
            Self::UnclassifiedMetric { origin } => {
                write!(formatter, "unclassified metric at {}", origin.pointer())
            }
            Self::MetricWithoutComponents { origin } => {
                write!(
                    formatter,
                    "metric without components at {}",
                    origin.pointer()
                )
            }
            Self::SeveralDatasets {
                first,
                second,
                origin,
            } => write!(
                formatter,
                "the bound query reads {} and {} (at {})",
                first.as_str(),
                second.as_str(),
                origin.pointer()
            ),
            Self::DuplicateHomeDataset => formatter.write_str("two branches share a home dataset"),
        }
    }
}

impl std::error::Error for InvariantViolation {}

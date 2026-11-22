//! The invariant violations of stage E3.

use std::fmt;

use menura_model::internal_model::NormalizedName;
use menura_representations::origin::QueryLocation;

/// A bound query that E2, or an earlier check of E3, never lets through.
///
/// This is an internal error of the compiler, not a specification diagnostic: no query that
/// passes those checks meets any of these conditions, so none has a specification code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvariantViolation {
    /// The query has neither metrics nor dimensions.
    NoMetricAndNoDimension,
    /// A `paths` entry names a relationship outside the authorized subgraph, which E2
    /// reports as `E_NAME_NOT_FOUND` (E2-4.8).
    UnauthorizedPathRelationship {
        /// Where the entry is written.
        origin: QueryLocation,
    },
    /// A `paths` entry whose relationships do not chain from its `from` to its `to`, which E2
    /// reports as `EQRY-12` (QRY-4.3).
    BrokenPathChain {
        /// Where the entry is written.
        origin: QueryLocation,
    },
    /// A metric that the model does not have, or that E0.g did not classify (E2-5 row 12a).
    UnclassifiedMetric {
        /// Where the metric is written.
        origin: QueryLocation,
    },
    /// A derived metric whose inputs lead back to itself, which E0 reports as `E0C-34`
    /// (DER-5.1.4).
    DerivedCycle {
        /// Where the metric is written.
        origin: QueryLocation,
    },
    /// A metric without components: neither aggregates of its own nor `AGG(…)` inputs.
    MetricWithoutComponents {
        /// Where the metric is written.
        origin: QueryLocation,
    },
    /// No path leads from the home dataset `home` to the needed dataset `dataset`, which
    /// E2-5 rows 13 to 16 report first.
    UnreachableDataset {
        /// The home dataset of the branch.
        home: NormalizedName,
        /// The needed dataset.
        dataset: NormalizedName,
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
            Self::UnauthorizedPathRelationship { origin } => write!(
                formatter,
                "the path at {} names an unauthorized relationship",
                origin.pointer()
            ),
            Self::BrokenPathChain { origin } => {
                write!(formatter, "the path at {} does not chain", origin.pointer())
            }
            Self::UnclassifiedMetric { origin } => {
                write!(formatter, "unclassified metric at {}", origin.pointer())
            }
            Self::DerivedCycle { origin } => {
                write!(formatter, "derived metric cycle at {}", origin.pointer())
            }
            Self::MetricWithoutComponents { origin } => {
                write!(
                    formatter,
                    "metric without components at {}",
                    origin.pointer()
                )
            }
            Self::UnreachableDataset { home, dataset } => write!(
                formatter,
                "no path leads from {} to {}",
                home.as_str(),
                dataset.as_str()
            ),
            Self::DuplicateHomeDataset => formatter.write_str("two branches share a home dataset"),
        }
    }
}

impl std::error::Error for InvariantViolation {}

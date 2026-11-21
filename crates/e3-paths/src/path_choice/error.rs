//! The errors of path choice (E3-3.2, E3-3.3, E3-6), each with its specification code and
//! its location in the query.

use std::fmt;

use menura_model::internal_model::NormalizedName;
use menura_representations::origin::QueryLocation;

/// The codes path choice reports, both at row 17 of E2-5, in this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PathChoiceCode {
    /// `E_AMBIGUOUS_PATH`: two or more paths lead to a needed dataset and the query chooses
    /// none (E3-3.2).
    AmbiguousPath,
    /// `E3-01`: a `paths` entry that no branch uses (E3-3.3).
    UnusedPathEntry,
}

impl PathChoiceCode {
    /// The code as the specification writes it.
    ///
    /// ```
    /// use menura_e3_paths::PathChoiceCode;
    /// assert_eq!(PathChoiceCode::UnusedPathEntry.as_str(), "E3-01");
    /// ```
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AmbiguousPath => "E_AMBIGUOUS_PATH",
            Self::UnusedPathEntry => "E3-01",
        }
    }
}

/// A failure of path choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathChoiceError {
    /// Two or more paths lead from `home` to `dataset`, which the element written at `origin`
    /// needs, and the query chooses none (E3-3.2).
    AmbiguousPath {
        /// The home dataset of the branch.
        home: NormalizedName,
        /// The needed dataset.
        dataset: NormalizedName,
        /// Where the element that needs it is written: a dimension, a filter or the
        /// `date_range` entry.
        origin: QueryLocation,
    },
    /// The `paths` entry from `from` to `to`, written at `origin`, that no branch uses
    /// (E3-3.3).
    UnusedPathEntry {
        /// Its `from`.
        from: NormalizedName,
        /// Its `to`.
        to: NormalizedName,
        /// Where it is written.
        origin: QueryLocation,
    },
    /// A bound query or a set of home datasets that the earlier stages never produce.
    Invariant(ChoiceInvariant),
}

impl PathChoiceError {
    /// The specification code; `None` for an invariant violation, which has none.
    #[must_use]
    pub fn code(&self) -> Option<PathChoiceCode> {
        match self {
            Self::AmbiguousPath { .. } => Some(PathChoiceCode::AmbiguousPath),
            Self::UnusedPathEntry { .. } => Some(PathChoiceCode::UnusedPathEntry),
            Self::Invariant(_) => None,
        }
    }

    /// Where the error is located in the query; `None` for an invariant violation.
    #[must_use]
    pub fn origin(&self) -> Option<&QueryLocation> {
        match self {
            Self::AmbiguousPath { origin, .. } | Self::UnusedPathEntry { origin, .. } => {
                Some(origin)
            }
            Self::Invariant(_) => None,
        }
    }
}

/// A condition that E2 and the reachability checks of E3 never let through.
///
/// This is an internal error of the compiler, not a specification diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChoiceInvariant {
    /// A `paths` entry names a relationship outside the authorized subgraph (E2-4.8).
    UnknownRelationship {
        /// Where the entry is written.
        origin: QueryLocation,
    },
    /// A `paths` entry whose relationships do not chain from `from` to `to` (QRY-4.3).
    BrokenChain {
        /// Where the entry is written.
        origin: QueryLocation,
    },
    /// No path leads from `home` to `dataset`, which rows 13 to 16 of E2-5 report first.
    Unreachable {
        /// The home dataset of the branch.
        home: NormalizedName,
        /// The needed dataset.
        dataset: NormalizedName,
    },
    /// No home dataset was given, or two share a name (E3-4.1).
    InvalidHomes,
}

impl fmt::Display for PathChoiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AmbiguousPath {
                home,
                dataset,
                origin,
            } => write!(
                formatter,
                "{}: more than one path leads from {} to {} (at {})",
                PathChoiceCode::AmbiguousPath.as_str(),
                home.as_str(),
                dataset.as_str(),
                origin.pointer()
            ),
            Self::UnusedPathEntry { from, to, origin } => write!(
                formatter,
                "{}: no branch uses the path from {} to {} (at {})",
                PathChoiceCode::UnusedPathEntry.as_str(),
                from.as_str(),
                to.as_str(),
                origin.pointer()
            ),
            Self::Invariant(invariant) => invariant.fmt(formatter),
        }
    }
}

impl fmt::Display for ChoiceInvariant {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownRelationship { origin } => write!(
                formatter,
                "the path at {} names an unauthorized relationship",
                origin.pointer()
            ),
            Self::BrokenChain { origin } => {
                write!(formatter, "the path at {} does not chain", origin.pointer())
            }
            Self::Unreachable { home, dataset } => write!(
                formatter,
                "no path leads from {} to {}",
                home.as_str(),
                dataset.as_str()
            ),
            Self::InvalidHomes => formatter.write_str("no home dataset, or a repeated one"),
        }
    }
}

impl std::error::Error for PathChoiceError {}

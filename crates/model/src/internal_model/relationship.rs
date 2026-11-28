//! Relationships and the graph of the usable ones (E0H-2 "Graph", E0H-2.3).

use super::common::{Descriptions, DiagnosticCode, NormalizedName, SourceLocation};
use super::validity::{OpenEnd, ValidToBound};

/// One relationship of the model, usable or not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relationship {
    /// Its name.
    pub name: NormalizedName,
    /// The dataset on the many side.
    pub from: NormalizedName,
    /// The dataset on the one side.
    pub to: NormalizedName,
    /// `from_columns`, in the order written.
    pub from_columns: Vec<NormalizedName>,
    /// `to_columns`, in the order written; paired by position with `from_columns`.
    pub to_columns: Vec<NormalizedName>,
    /// Its `temporal` entry when it is a temporal relationship (VAL-2.1); `None` for a plain
    /// one.
    pub temporal: Option<TemporalJoin>,
    /// Its `description`, `label` and `ai_context`.
    pub descriptions: Descriptions,
    /// The query error codes of a query that names it, ordered by code; empty when it is
    /// usable (E0-4.1, E0-4.2a).
    pub query_errors: Vec<DiagnosticCode>,
    /// Where it is written.
    pub location: SourceLocation,
}

/// The `temporal` entry of a temporal relationship (VAL-2.1): each row of `from` joins the
/// versions of `to` that the population covers at its `from_time` value (VAL-2.8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalJoin {
    /// The field of `from` that holds the time of each row (VAL-2.2).
    pub from_time: NormalizedName,
    /// The `validity` population of `to` that supplies the versions (VAL-2.3).
    pub population: NormalizedName,
}

/// The fields a temporal relationship compares (VAL-2.3, VAL-2.7): its `from_time` and the
/// range of its population, as [`crate::internal_model::InternalModel::temporal_range`]
/// resolves them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TemporalRange<'model> {
    /// The field of `from` that holds the time of each row.
    pub from_time: &'model NormalizedName,
    /// The population that supplies the versions.
    pub population: &'model NormalizedName,
    /// The field of `to` of the start of validity.
    pub valid_from: &'model NormalizedName,
    /// The field of `to` of the end of validity.
    pub valid_to: &'model NormalizedName,
    /// Whether `valid_to` is the last value covered or the first one not covered (VAL-1.1).
    pub valid_to_bound: ValidToBound,
    /// The `valid_to` value that reads like NULL, when declared (VAL-1.2).
    pub open_end: Option<&'model OpenEnd>,
}

/// The directed graph of the usable relationships, with what each dataset reaches (E0H-2.3).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RelationshipGraph {
    /// One edge per usable relationship between two different datasets, ordered by
    /// relationship name.
    pub edges: Vec<GraphEdge>,
    /// For each dataset of the graph, ordered by name, the datasets it reaches.
    pub reachability: Vec<DatasetReach>,
    /// Every pair of datasets with more than one path, ordered by `from`, then `to`.
    pub several_paths: Vec<DatasetPair>,
}

/// An edge of the graph: one relationship, from the many side to the one side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphEdge {
    /// The relationship, which names the edge.
    pub relationship: NormalizedName,
    /// The dataset on the many side.
    pub from: NormalizedName,
    /// The dataset on the one side.
    pub to: NormalizedName,
    /// Whether `from_columns` also matches a key of `from` (E0E-3.8).
    pub one_to_one: bool,
}

/// The datasets one dataset reaches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetReach {
    /// The dataset paths start from.
    pub dataset: NormalizedName,
    /// The datasets reached, ordered by name.
    pub reached: Vec<ReachedDataset>,
}

/// A dataset reached, with how many paths lead there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReachedDataset {
    /// The dataset reached.
    pub dataset: NormalizedName,
    /// How many paths lead there.
    pub paths: PathCount,
}

/// How many paths lead from one dataset to another (E0E-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PathCount {
    /// Exactly one.
    One,
    /// More than one (E0E-4.1).
    Several,
}

/// An ordered pair of datasets.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DatasetPair {
    /// Where paths start.
    pub from: NormalizedName,
    /// Where paths end.
    pub to: NormalizedName,
}

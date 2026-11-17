//! The result of relationship analysis (E0E-6), as data types only.

use super::diagnostic::RelationshipDiagnostic;
use crate::names::Name;
use crate::pointer::JsonPointer;

/// The index of a dataset in [`RelationshipGraph::datasets`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct DatasetIndex(pub(crate) usize);

/// The relationships of a model that loads: the graph, what each dataset reaches, the pairs
/// with more than one path, the relationships excluded as invalid, and the warnings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Relationships {
    /// The directed graph with named edges (E0E-2).
    pub(crate) graph: RelationshipGraph,
    /// For each dataset of the graph, by [`DatasetIndex`]: the datasets it reaches, in graph
    /// order, each with whether one path or more lead there (E0E-6).
    pub(crate) reachable: Vec<Vec<Reachable>>,
    /// Every pair of datasets with more than one path, by source then target in graph order
    /// (E0E-4.2).
    pub(crate) several_paths: Vec<DatasetPair>,
    /// The relationships made unusable by `E0E-01` or `E0E-02`, in document order (E0-4.1).
    /// A query that names one fails with
    /// [`RelationshipCode::INVALID_RELATIONSHIP_QUERY_ERROR`](super::RelationshipCode::INVALID_RELATIONSHIP_QUERY_ERROR).
    pub(crate) unusable: Vec<JsonPointer>,
    /// Every diagnostic, in document order then by code (E0-4.3); `W0E-01` diagnostics with the
    /// same `from` dataset are ordered by their target dataset in document order (E0E-4.2).
    pub(crate) warnings: Vec<RelationshipDiagnostic>,
}

/// The directed graph of the relationships (E0E-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RelationshipGraph {
    /// The nodes: every dataset of the model except summaries, in document order (E0E-2.1).
    pub(crate) datasets: Vec<GraphDataset>,
    /// The edges: one per usable relationship between two different datasets, in document
    /// order, directed from the many side to the one side (E0E-2.2, E0E-2.3).
    pub(crate) edges: Vec<Edge>,
}

/// A node of the graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphDataset {
    /// The dataset name.
    pub(crate) name: Name,
    /// The JSON Pointer of the dataset.
    pub(crate) pointer: JsonPointer,
}

/// An edge of the graph: one relationship.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Edge {
    /// The relationship name, which identifies the edge (E0E-2.2).
    pub(crate) name: Name,
    /// The JSON Pointer of the relationship.
    pub(crate) pointer: JsonPointer,
    /// The dataset on the many side.
    pub(crate) from: DatasetIndex,
    /// The dataset on the one side.
    pub(crate) to: DatasetIndex,
    /// Whether `from_columns` also matches a key of `from` (E0E-3.8), so the relationship is
    /// one-to-one; it is still traversed only from `from` to `to` (E0E-3.4).
    pub(crate) one_to_one: bool,
}

/// How many paths lead from one dataset to another (E0E-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum PathCount {
    /// Exactly one path.
    One,
    /// More than one path (E0E-4.1).
    Several,
}

/// A dataset reached from another one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Reachable {
    /// The dataset reached.
    pub(crate) dataset: DatasetIndex,
    /// How many paths lead there.
    pub(crate) paths: PathCount,
}

/// An ordered pair of datasets: paths start at `from` and end at `to`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct DatasetPair {
    /// The dataset paths start from.
    pub(crate) from: DatasetIndex,
    /// The dataset paths end at.
    pub(crate) to: DatasetIndex,
}

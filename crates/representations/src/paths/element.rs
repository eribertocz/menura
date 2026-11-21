//! The query elements a join tree binds to its join instances (E3-4.2a).

use menura_model::internal_model::NormalizedName;

use super::JoinStep;

/// A query element that reads a dataset through a path: E3 binds it to the join instance at
/// the end of its own path (E3-4.2a).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QueryElement {
    /// The dimension at this position among the dimensions of the bound query, in output
    /// order, from 0.
    Dimension(usize),
    /// The filter at position `filter` among the filters of the bound query, from 0, where it
    /// reads `dataset`: a filter that reads several datasets has one path to each.
    Filter {
        /// The position of the filter, from 0.
        filter: usize,
        /// One of the datasets it reads.
        dataset: NormalizedName,
    },
    /// The `date_range` of the query.
    DateRange,
}

/// The path of one query element from the home dataset of its branch (E3-3.1). An empty path
/// means that the element reads the home dataset itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementPath {
    /// The element.
    pub element: QueryElement,
    /// Its steps from the home dataset, in order.
    pub steps: Vec<JoinStep>,
}

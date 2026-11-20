//! Summaries: their mapping to their base and their definition hash (E0H-2 "Summaries",
//! E0H-3.3).

use menura_contracts::Sha256Hash;
use menura_contracts::types::common::Grain;

use super::common::{NormalizedName, SourceLocation};
use crate::expression::Expression;

/// One summary dataset and how it maps to its base (SUM-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    /// The summary dataset.
    pub dataset: NormalizedName,
    /// The base dataset (`of`).
    pub of: NormalizedName,
    /// The dimensions, in the order written.
    pub dimensions: Vec<SummaryDimension>,
    /// The measures, in the order written.
    pub measures: Vec<SummaryMeasure>,
    /// The summary field whose coverage the catalog reports.
    pub coverage_field: NormalizedName,
    /// The summary definition hash (E0H-3.3).
    pub definition_hash: Sha256Hash,
    /// Where the entry of ours that declares it is written.
    pub location: SourceLocation,
}

/// One dimension of a summary (SUM-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryDimension {
    /// The summary field.
    pub field: NormalizedName,
    /// The base field it groups by.
    pub base: NormalizedName,
    /// The grain of a time dimension.
    pub grain: Option<Grain>,
}

/// One measure of a summary (SUM-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryMeasure {
    /// The summary field.
    pub field: NormalizedName,
    /// The normalized tree of `base_aggregate`.
    pub base_aggregate: Option<Expression>,
}

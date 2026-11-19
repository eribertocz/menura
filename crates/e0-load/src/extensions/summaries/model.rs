//! The declarations of the summary extension (SUM-2), as data types only.

use crate::pointer::JsonPointer;
use crate::read::Location;

/// The grain of a time dimension of a summary (SUM-6.1.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grain {
    /// `day`.
    Day,
    /// `week`.
    Week,
    /// `month`.
    Month,
    /// `quarter`.
    Quarter,
    /// `year`.
    Year,
}

impl Grain {
    /// Every grain with its written value.
    pub(crate) const ALL: [(&'static str, Self); 5] = [
        ("day", Self::Day),
        ("week", Self::Week),
        ("month", Self::Month),
        ("quarter", Self::Quarter),
        ("year", Self::Year),
    ];
}

/// One dimension of a summary: a summary field and the base field it groups by (SUM-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryDimension {
    /// The summary field, as written.
    pub field: String,
    /// The field of the `of` dataset, as written.
    pub base: String,
    /// The grain, written only on a time dimension (SUM-6.1.4).
    pub grain: Option<Grain>,
}

/// One measure of a summary: a summary field and the aggregate that produced it (SUM-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryMeasure {
    /// The summary field, as written.
    pub field: String,
    /// The text of `base_aggregate`, parsed and typed by E0.f (SUM-6.1.3).
    pub base_aggregate: String,
}

/// The `aggregate` of one summary dataset, whose shape is valid (SUM-6.1.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryDeclaration {
    /// The JSON Pointer of the summary dataset.
    pub dataset: JsonPointer,
    /// The JSON Pointer of the `data` member of the dataset's entry of ours, where every
    /// diagnostic about the summary is located (E0C-2.6).
    pub data: JsonPointer,
    /// Where that `data` member starts in the document text.
    pub location: Location,
    /// The base dataset, as written.
    pub of: String,
    /// The dimensions, at least one, in the order written.
    pub dimensions: Vec<SummaryDimension>,
    /// The measures, at least one, in the order written.
    pub measures: Vec<SummaryMeasure>,
    /// The summary field whose coverage the catalog reports, as written.
    pub coverage_field: String,
}

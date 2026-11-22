//! The conditions for the use of a summary (SUM-3) that do not need the catalog state.

/// A condition of SUM-3 that a summary fails for a branch, in the order they are checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum SummaryCondition {
    /// SUM-3.1: a component matches no summary measure, or does not recompose exactly.
    Equivalence,
    /// SUM-3.2: a declared non-additive or semi-additive metric, a remaining `Deduplicate`, or a
    /// float argument.
    Exclusions,
    /// SUM-3.2a: a summary row depends on the time or time zone of its build.
    QueryDependentValues,
    /// SUM-3.3: a query dimension, filter or `date_range` field the summary does not keep.
    Grain,
    /// SUM-3.3a: a truncated summary field read other than by its time grain or date range.
    TruncatedFields,
    /// SUM-3.4: a query time grain that does not derive from the summary grain.
    TimeGrain,
    /// SUM-3.5: a date range not aligned on the summary grain.
    TimeFilterAlignment,
    /// SUM-3.6: a row policy of the base that cannot be applied to the summary.
    PolicyPreservation,
    /// SUM-3.8: a summary field read without a logical type.
    SameType,
}

impl SummaryCondition {
    /// The rule ID of the condition, as a rejected rewrite records it (E5-3.8).
    pub(crate) fn rule_id(self) -> &'static str {
        match self {
            Self::Equivalence => "SUM-3.1",
            Self::Exclusions => "SUM-3.2",
            Self::QueryDependentValues => "SUM-3.2a",
            Self::Grain => "SUM-3.3",
            Self::TruncatedFields => "SUM-3.3a",
            Self::TimeGrain => "SUM-3.4",
            Self::TimeFilterAlignment => "SUM-3.5",
            Self::PolicyPreservation => "SUM-3.6",
            Self::SameType => "SUM-3.8",
        }
    }
}

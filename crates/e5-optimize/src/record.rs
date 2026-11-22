//! The records of the rewrites E5 applied or rejected (E5-3.8, E5-4.4): data only.

use menura_model::internal_model::NormalizedName;

/// One rewrite of E5-3, named by its rule ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RewriteRule {
    /// E5-3.1: remove unused joins.
    RemoveUnusedJoins,
    /// E5-3.2: skip deduplication.
    SkipDeduplication,
    /// E5-3.3: change `left` joins to `inner`.
    InnerJoins,
    /// E5-3.4: merge scans of identical branches.
    MergeScans,
    /// E5-3.5: use a summary, selected as E5-4 states.
    UseSummary,
    /// E5-3.6: fold constants.
    FoldConstants,
}

impl RewriteRule {
    /// The rule ID of the rewrite, such as `E5-3.1`.
    ///
    /// # Example
    ///
    /// ```
    /// use menura_e5_optimize::RewriteRule;
    ///
    /// assert_eq!(RewriteRule::InnerJoins.rule_id(), "E5-3.3");
    /// ```
    #[must_use]
    pub fn rule_id(self) -> &'static str {
        match self {
            Self::RemoveUnusedJoins => "E5-3.1",
            Self::SkipDeduplication => "E5-3.2",
            Self::InnerJoins => "E5-3.3",
            Self::MergeScans => "E5-3.4",
            Self::UseSummary => "E5-3.5",
            Self::FoldConstants => "E5-3.6",
        }
    }
}

/// The position of a node in a plan: the index of each input taken from the root, in the order
/// of `PlanNode::inputs`. The root is the empty path.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodePath(Vec<usize>);

impl NodePath {
    /// The path that takes the input indexes `steps` from the root.
    #[must_use]
    pub fn new(steps: Vec<usize>) -> Self {
        Self(steps)
    }

    /// The input indexes from the root.
    #[must_use]
    pub fn steps(&self) -> &[usize] {
        &self.0
    }
}

/// What a rewrite record concerns.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RewriteSubject {
    /// A node of the input plan of the rewrite.
    Node(NodePath),
    /// A summary dataset, by its normalized name.
    Summary(NormalizedName),
}

/// Whether a considered rewrite was applied.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RewriteOutcome {
    /// The rewrite was applied.
    Applied,
    /// The rewrite was rejected.
    Rejected {
        /// The condition that failed, as the rule states it.
        failed_condition: String,
    },
}

/// One rewrite considered by E5: applied, or rejected with its failed condition (E5-3.8).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RewriteRecord {
    /// The rewrite.
    pub rule: RewriteRule,
    /// The node or summary it concerns.
    pub subject: RewriteSubject,
    /// Whether it was applied.
    pub outcome: RewriteOutcome,
}

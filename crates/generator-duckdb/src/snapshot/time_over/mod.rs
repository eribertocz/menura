//! A `Snapshot` across a time `over` (E4-7, ADD-4): the components per (result group, entity,
//! period), the `unexpected`, `nulls` and `missing` rules, then one value per (result group,
//! entity) under `aggregation`. Each runtime condition of ADD-7 is raised in SQL (E6-4.6b).

mod fill;
mod frame;
mod missing;
mod reduce;
mod values;

use menura_model::internal_model::{SnapshotAggregation, UnexpectedHandling};
use menura_representations::logical_plan::{PlanNode, SnapshotMissing, SnapshotRule, TimeSnapshot};

use crate::body::SelectBody;
use crate::error::GenerateError;
use crate::steps::StepWriter;
use frame::Frame;

/// The steps a `Snapshot` reads, by name: its `values`, after the rules (`checked`), the
/// pairs `missing` finds, and the rows `fill` adds for them.
struct Steps {
    values: String,
    checked: String,
    missing: Option<String>,
    filled: Option<String>,
}

impl<'input> StepWriter<'input> {
    /// The steps of `input`, `expected` and the `carried` or `added` rows, in that order
    /// (E6-4.12), then the steps of the rule; the body returned is the last.
    pub(super) fn time_snapshot(
        &mut self,
        node: &'input PlanNode,
        input: &'input PlanNode,
        rule: &'input SnapshotRule,
        time: &'input TimeSnapshot,
    ) -> Result<SelectBody, GenerateError> {
        let written = self.write(input)?;
        let (expected, periods) = self.write_population(&time.expected)?;
        let extra = match &time.missing {
            SnapshotMissing::CarryForward { carried, .. } => Some(self.write(carried)?.name),
            SnapshotMissing::Zero { added } => Some(self.write(added)?.name),
            SnapshotMissing::Exclude | SnapshotMissing::Error => None,
        };
        self.parameters.start_node(node.origin());
        let frame = Frame::new(node, rule, time, expected, periods)?;
        let values = self.period_values(&frame, input, &written.name, None)?;
        let values = self.push_step("values", values);
        let checked = values::checked(&frame, &values);
        let checked = self.push_step("checked", checked);
        let missing = self.missing(&frame, &checked);
        let filled = match (&missing, extra) {
            (Some(missing), Some(extra)) => Some(self.fill(&frame, missing, &extra)?),
            _ => None,
        };
        let steps = Steps {
            values,
            checked,
            missing,
            filled,
        };
        self.reduce(&frame, &steps)
    }
}

/// Whether `rule` reads the expected pairs to find unexpected rows (ADD-4.4).
fn reads_unexpected(time: &TimeSnapshot) -> bool {
    time.unexpected != UnexpectedHandling::Include
}

/// The `selection: per_entity` edge of `aggregation`: `MAX` for `last`, `MIN` for `first`,
/// `None` otherwise (ADD-4.1, E4-7.7).
fn per_entity_edge(aggregation: SnapshotAggregation) -> Option<&'static str> {
    use menura_model::internal_model::Selection;
    match aggregation {
        SnapshotAggregation::Last(Selection::PerEntity) => Some("MAX"),
        SnapshotAggregation::First(Selection::PerEntity) => Some("MIN"),
        _ => None,
    }
}

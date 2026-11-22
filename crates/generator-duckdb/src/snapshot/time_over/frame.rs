//! The names a time `Snapshot` writes its steps with.

use menura_representations::logical_plan::{PlanNode, SnapshotRule, TimeSnapshot};

use crate::error::{GenerateError, InvariantViolation};
use crate::identifiers::quote;
use crate::steps::error;

/// The synthesized column of the period of a value (E6-4.12).
pub(super) const PERIOD: &str = "\"menura.period\"";
/// The synthesized column of the unit of a period (ADD-4.7).
pub(super) const UNIT: &str = "\"menura.unit\"";
/// The synthesized column that marks a stale carried value (ADD-4.3).
pub(super) const STALE: &str = "\"menura.stale\"";
/// The alias of an expected pair.
pub(super) const EXPECTED: &str = "\"menura_expected\"";
/// The synthesized column of the text of a runtime condition (ADD-7).
pub(super) const VIOLATION: &str = "\"menura.violation\"";

/// The days of the periods a time `Snapshot` reads (POP-3.5): of an expected pair, and of
/// the query's periods, with the step that holds them.
pub(super) struct Days {
    pub(super) expected: String,
    pub(super) periods: String,
    pub(super) period: String,
}

/// A `Snapshot` node with a time `over`, and the names its steps read.
pub(super) struct Frame<'input> {
    pub(super) node: &'input PlanNode,
    pub(super) rule: &'input SnapshotRule,
    pub(super) time: &'input TimeSnapshot,
    /// The quoted key columns, in order.
    pub(super) keys: Vec<String>,
    /// The quoted entity columns, in order.
    pub(super) entity: Vec<String>,
    /// The quoted component columns, in order.
    pub(super) components: Vec<String>,
    /// The step of the expected pairs.
    pub(super) expected: String,
    /// Its quoted entity columns, paired in order with `entity`.
    pub(super) expected_entity: Vec<String>,
    /// Its quoted period column.
    pub(super) expected_period: String,
    /// The day of the period of an expected pair, `menura_expected` (POP-3.5).
    pub(super) expected_day: String,
    /// The step of the query's periods, and the day of its period column.
    pub(super) periods: (String, String),
}

impl<'input> Frame<'input> {
    /// The frame of `node`, whose `expected` step is `expected` and the step of its periods
    /// `periods`.
    pub(super) fn new(
        node: &'input PlanNode,
        rule: &'input SnapshotRule,
        time: &'input TimeSnapshot,
        expected: String,
        days: Days,
    ) -> Result<Self, GenerateError> {
        let expected_columns = time.expected.schema().columns();
        let Some((expected_period, expected_entity)) = expected_columns.split_last() else {
            return Err(error(InvariantViolation::UnknownColumn, node));
        };
        if expected_entity.len() != rule.entity.len() {
            return Err(error(InvariantViolation::UnknownColumn, node));
        }
        Ok(Self {
            node,
            rule,
            time,
            keys: names(rule.keys.iter().map(|key| key.name.as_str())),
            entity: names(rule.entity.iter().map(|item| item.name.as_str())),
            components: names(rule.components.iter().map(|item| item.name.as_str())),
            expected,
            expected_entity: names(expected_entity.iter().map(|column| column.name.as_str())),
            expected_period: quote(expected_period.name.as_str()),
            expected_day: days.expected,
            periods: (days.periods, days.period),
        })
    }

    /// The keys, then the entity, then the period, then the components.
    pub(super) fn value_columns(&self) -> Vec<String> {
        let mut columns: Vec<String> = self.keys.iter().chain(&self.entity).cloned().collect();
        columns.push(PERIOD.to_owned());
        columns.extend(self.components.iter().cloned());
        columns
    }
}

fn names<'name>(names: impl Iterator<Item = &'name str>) -> Vec<String> {
    names.map(quote).collect()
}

/// Each of `columns` qualified by `alias`.
pub(super) fn qualified(alias: &str, columns: &[String]) -> Vec<String> {
    columns
        .iter()
        .map(|column| format!("{alias}.{column}"))
        .collect()
}

/// `left.l1 = right.r1 AND …`, one equality per pair, in order.
pub(super) fn equalities(
    left: &str,
    left_columns: &[String],
    right: &str,
    right_columns: &[String],
) -> String {
    left_columns
        .iter()
        .zip(right_columns)
        .map(|(left_column, right_column)| format!("{left}.{left_column} = {right}.{right_column}"))
        .collect::<Vec<_>>()
        .join(" AND ")
}

/// `CAST(date_trunc('<grain>', period) AS DATE)`: the start of the period at `grain` that
/// holds `period` (E6-5.2).
pub(super) fn truncated(period: &str, grain: &str) -> String {
    format!("CAST(date_trunc('{grain}', {period}) AS DATE)")
}

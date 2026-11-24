//! What the checks that are not a single plan read (CHK-6.3–CHK-6.5): the plan of their rows,
//! the columns of each role and the names of their result columns (CHK-2.6).

use menura_contracts::types::common::Grain;
use menura_representations::logical_plan::{PlanNode, PopulationRule};
use menura_representations::names::ColumnName;

/// The `population` check with `mode: validity` (CHK-6.3).
#[derive(Debug, Clone)]
pub struct ValidityCheck<'plan> {
    /// Every row of the dataset that declares the population, with no row policy (CHK-2.2).
    pub rows: &'plan PlanNode,
    /// The columns of `rows` of the `entity` fields, in declared order.
    pub entity: Vec<ColumnName>,
    /// The column of `rows` of `valid_from`.
    pub valid_from: ColumnName,
    /// The column of `rows` of `valid_to`.
    pub valid_to: ColumnName,
    /// The result column names: the entity fields, `valid_from`, `valid_to`, then
    /// `violation` (CHK-2.6).
    pub result_columns: Vec<String>,
}

/// The `population` check with `mode: periodic` (CHK-6.4).
#[derive(Debug, Clone)]
pub struct PeriodicCheck<'plan> {
    /// Every row of the dataset that declares the population, with no row policy (CHK-2.2).
    pub rows: &'plan PlanNode,
    /// The columns of `rows` of the `entity` fields, in declared order.
    pub entity: Vec<ColumnName>,
    /// The column of `rows` of `period`.
    pub period: ColumnName,
    /// The grain of `period`.
    pub grain: Grain,
    /// The result column names: the entity fields, `period`, then `row_count` (CHK-2.6).
    pub result_columns: Vec<String>,
}

/// The `semi_additive` check of a metric (CHK-6.5).
#[derive(Debug, Clone)]
pub struct SnapshotCheck<'plan> {
    /// Every row of the metric's home dataset, with no row policy (CHK-2.2).
    pub rows: &'plan PlanNode,
    /// The columns of `rows` of the `entity` fields, in declared order.
    pub entity: Vec<ColumnName>,
    /// The column of `rows` of `over`.
    pub over: ColumnName,
    /// With a time `over`, its grain and population; `None` for any other `over`, which has
    /// only `duplicate_snapshot` (ADD-5.1.5).
    pub time: Option<SnapshotTime<'plan>>,
    /// The result column names: the entity fields, `over`, then `violation` (CHK-2.6).
    pub result_columns: Vec<String>,
}

/// The time part of a `semi_additive` check: its `snapshot_grain` and its population.
#[derive(Debug, Clone)]
pub struct SnapshotTime<'plan> {
    /// The `snapshot_grain`.
    pub grain: Grain,
    /// Every row of the dataset that declares the population, with no row policy (CHK-2.2).
    pub population: &'plan PlanNode,
    /// When an entity is expected, over the columns of `population` (POP-3).
    pub rule: &'plan PopulationRule,
}

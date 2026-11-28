//! What the integrity checks that are not a single plan read (CHK-6.3–CHK-6.5, VAL-2.16a): the
//! plan of their rows, the columns of each role and the names of their result columns
//! (CHK-2.6). Both dialect generators take these shapes (E6-6.2, E6-7.11).

use crate::logical_plan::{PlanNode, PopulationRule};
use crate::names::ColumnName;
use menura_contracts::types::common::{Grain, LogicalType};
use menura_model::internal_model::{OpenEnd, ValidToBound};

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
    /// Whether `valid_to` is inclusive, compared by days, or exclusive, compared by values
    /// (CHK-6.3, VAL-1.4).
    pub valid_to_bound: ValidToBound,
    /// The `valid_to` value that reads like NULL, when declared (VAL-1.2).
    pub open_end: Option<OpenEnd>,
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

/// The `temporal_orphans` check of a temporal relationship (VAL-2.16a): the rows of `from`
/// whose keys and time are all non-NULL and that no version of `to` covers (VAL-2.7, VAL-2.8).
#[derive(Debug, Clone)]
pub struct TemporalOrphansCheck<'plan> {
    /// Every row of the `from` dataset, with no row policy (CHK-2.2).
    pub rows: &'plan PlanNode,
    /// Every row of the `to` dataset, with no row policy (CHK-2.2).
    pub versions: &'plan PlanNode,
    /// The key pairs, in pair order: a column of `rows` (`from_columns`) and the column of
    /// `versions` (`to_columns`) it must equal.
    pub keys: Vec<(ColumnName, ColumnName)>,
    /// The column of `rows` of `from_time`.
    pub from_time: ColumnName,
    /// The time range of each version, over the columns of `versions`.
    pub range: VersionRange,
    /// The type `from_time`, `valid_from` and `valid_to` are compared as (VAL-2.9): their
    /// common kind, at the finest precision among them for timestamps, so that a value of a
    /// coarser precision is compared exactly, never rounded. A value of another type is cast
    /// to it.
    pub compared_as: LogicalType,
    /// The result columns, in order: a column of `rows` and the name it is returned under
    /// (CHK-2.6): the `from_columns` fields in pair order, then `from_time` unless it is one
    /// of them.
    pub result_columns: Vec<(ColumnName, String)>,
}

/// The time range of the versions of a temporal relationship: the `valid_from`, `valid_to`,
/// `valid_to_bound` and `open_end` of its population (VAL-2.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionRange {
    /// The column of `valid_from`.
    pub valid_from: ColumnName,
    /// The column of `valid_to`.
    pub valid_to: ColumnName,
    /// Whether `valid_to` is the last value covered or the first one not covered (VAL-1.1).
    pub valid_to_bound: ValidToBound,
    /// The `valid_to` value that reads like NULL, when declared (VAL-1.2).
    pub open_end: Option<OpenEnd>,
}

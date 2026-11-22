//! The expected-entity rule a `Population` node applies (E4-3.12, POP-3).

use menura_model::expression::Expression;
use menura_model::internal_model::PartialPeriod;

use crate::names::ColumnName;

/// When an entity of a population is expected in a period (POP-3), over the columns of the
/// visible rows the `Population` node reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopulationRule {
    /// The columns that identify the entity, in the order the population writes them.
    pub entity: Vec<ColumnName>,
    /// The mode of the population, over the columns of its rows.
    pub membership: Membership,
    /// The `required_when` condition, over the columns of its rows, when written: the entity
    /// is expected only where it holds (POP-3.3).
    pub required_when: Option<Expression>,
}

/// The `mode` of a population, with the columns it reads (POP-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Membership {
    /// `validity`: an entity is expected in a period its validity covers according to
    /// `partial_period` (POP-3.1). An entity may have several rows (POP-3.4).
    Validity {
        /// The column of the start of validity, inclusive.
        valid_from: ColumnName,
        /// The column of the end of validity, inclusive; NULL means open-ended.
        valid_to: ColumnName,
        /// How much of the period the validity must cover.
        partial_period: PartialPeriod,
    },
    /// `periodic`: an entity is expected in a period if its row for that period exists
    /// (POP-3.2). Its grain is the node's `grain`, which E0 requires (ADD-5.1.7).
    Periodic {
        /// The column of the period of the row.
        period: ColumnName,
    },
}

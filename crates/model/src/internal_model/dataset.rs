//! Datasets, their fields and populations (E0H-2 "Datasets").

use menura_contracts::types::common::Grain;

use super::common::{Descriptions, DiagnosticCode, NormalizedName, SourceLocation, TypedValue};
use super::metric::Metric;
use super::security::{DatasetAccess, RowPolicy};
use super::validity::{OpenEnd, ValidToBound};
use crate::expression::Expression;

/// One dataset of the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dataset {
    /// Its name.
    pub name: NormalizedName,
    /// Its `source`, as written: a catalog relation or a query.
    pub source: String,
    /// Its fields, ordered by name (E0H-1.4).
    pub fields: Vec<Field>,
    /// Its dataset-scoped metrics (development format, section 4.5), ordered by name.
    pub metrics: Vec<Metric>,
    /// The fields of `primary_key`, in the order written; empty when it has none.
    pub primary_key: Vec<NormalizedName>,
    /// Each `unique_keys` entry, with its fields in the order written, in the order written.
    pub unique_keys: Vec<Vec<NormalizedName>>,
    /// Its access, with its field access (SCY-3); `None` when it is inaccessible (SCY-3.0).
    pub access: Option<DatasetAccess>,
    /// Its row policies (SCY-4), ordered by name.
    pub row_policies: Vec<RowPolicy>,
    /// Its populations (POP-2), ordered by name.
    pub populations: Vec<Population>,
    /// Whether it is a summary (extension summaries).
    pub summary: bool,
    /// Its `description`, `label` and `ai_context`.
    pub descriptions: Descriptions,
    /// The query error codes of a query that names it, ordered by code; empty when it is
    /// usable (E0-4.1).
    pub query_errors: Vec<DiagnosticCode>,
    /// Where it is written.
    pub location: SourceLocation,
}

/// One field of a dataset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// Its name.
    pub name: NormalizedName,
    /// Its normalized expression tree (E0H-1.3), when its compiled variant parses.
    pub expression: Option<Expression>,
    /// Its type and nullability from the admitted catalog schema (E0H-2.2), when E0.f typed it.
    pub typed: Option<TypedValue>,
    /// Its `dimension.is_time`, when written.
    pub is_time: Option<bool>,
    /// Its `description`, `label` and `ai_context`.
    pub descriptions: Descriptions,
    /// The query error codes of a query that names it, ordered by code; empty when it is
    /// usable (E0-4.1, E0-4.2a).
    pub query_errors: Vec<DiagnosticCode>,
    /// Where it is written.
    pub location: SourceLocation,
}

/// One population of a dataset (POP-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Population {
    /// Its name, normalized (E0D-1.4): unique in the model (POP-4.3).
    pub name: NormalizedName,
    /// The fields that identify the entity, in the order written.
    pub entity: Vec<NormalizedName>,
    /// The mode and its fields.
    pub mode: PopulationMode,
    /// The normalized tree of `required_when`, when written.
    pub required_when: Option<Expression>,
    /// Where the entry of ours that declares it is written.
    pub location: SourceLocation,
}

/// The `mode` of a population, with its keys (POP-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopulationMode {
    /// `validity`: validity intervals.
    Validity {
        /// The field of the start of validity, inclusive.
        valid_from: NormalizedName,
        /// The field of the end of validity, inclusive unless `valid_to_bound` says otherwise.
        valid_to: NormalizedName,
        /// When an entity is expected in a period its validity covers partly.
        partial_period: PartialPeriod,
        /// Whether `valid_to` is the last value covered or the first one not covered
        /// (VAL-1.1).
        valid_to_bound: ValidToBound,
        /// The `valid_to` value that means open-ended, like NULL, when declared (VAL-1.2).
        open_end: Option<OpenEnd>,
    },
    /// `periodic`: one row per entity and period.
    Periodic {
        /// The period field.
        period: NormalizedName,
        /// The grain of `period`.
        grain: Grain,
    },
}

/// The values of `partial_period` (POP-2, POP-3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartialPeriod {
    /// `any_day`.
    AnyDay,
    /// `full_period`.
    FullPeriod,
    /// `period_end`.
    PeriodEnd,
}

impl PartialPeriod {
    /// The value as the extension writes it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AnyDay => "any_day",
            Self::FullPeriod => "full_period",
            Self::PeriodEnd => "period_end",
        }
    }
}

impl Population {
    /// Every field the population reads, in the order written: `entity`, then its time
    /// fields (POP-4.3); `required_when` is read separately.
    #[must_use]
    pub fn named_fields(&self) -> Vec<&NormalizedName> {
        let mut fields: Vec<&NormalizedName> = self.entity.iter().collect();
        match &self.mode {
            PopulationMode::Validity {
                valid_from,
                valid_to,
                ..
            } => fields.extend([valid_from, valid_to]),
            PopulationMode::Periodic { period, .. } => fields.push(period),
        }
        fields
    }
}

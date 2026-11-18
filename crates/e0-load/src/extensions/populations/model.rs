//! A population of the `populations` extension (POP-2), as data types only.

use semantic_compiler_contracts::types::common::Grain;

use crate::pointer::JsonPointer;
use crate::read::Location;

/// One item of a `populations` list whose shape and keys are valid (POP-4.1, POP-4.2).
///
/// Field names are kept as written: they are declared names (E0D-1.4) of fields of the
/// dataset that declares the population, resolved by E0.d (POP-4.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Population {
    /// The JSON Pointer of the dataset that declares the population.
    pub dataset: JsonPointer,
    /// The JSON Pointer of the `data` member of the dataset's entry of ours, where every
    /// diagnostic about the population is located (E0C-2.6, POP-4.6).
    pub data: JsonPointer,
    /// Where that `data` member starts in the document text.
    pub location: Location,
    /// The value of `name`.
    pub name: String,
    /// The fields of `entity`, in order, without repeats.
    pub entity: Vec<String>,
    /// The mode and its keys.
    pub mode: PopulationMode,
    /// The text of `required_when`, when given; parsed by E0.d and E0.f (POP-4.4).
    pub required_when: Option<String>,
}

/// The `mode` of a population, with the keys of that mode (POP-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopulationMode {
    /// `validity`: validity intervals.
    Validity {
        /// The field of the start of validity, inclusive.
        valid_from: String,
        /// The field of the end of validity, inclusive.
        valid_to: String,
        /// When an entity is expected in a period its validity covers partly.
        partial_period: PartialPeriod,
    },
    /// `periodic`: one row per entity and period.
    Periodic {
        /// The period field of the row.
        period: String,
        /// The grain of `period`.
        grain: Grain,
    },
}

/// The values of `partial_period` (POP-2, POP-3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartialPeriod {
    /// `any_day`: the validity covers at least one day of the period.
    AnyDay,
    /// `full_period`: the validity covers every day of the period.
    FullPeriod,
    /// `period_end`: the validity covers the last day of the period.
    PeriodEnd,
}

impl Population {
    /// The time fields of the population, as written: `valid_from` and `valid_to`, or
    /// `period` (POP-4.5).
    #[must_use]
    pub fn time_fields(&self) -> Vec<&str> {
        match &self.mode {
            PopulationMode::Validity {
                valid_from,
                valid_to,
                ..
            } => vec![valid_from, valid_to],
            PopulationMode::Periodic { period, .. } => vec![period],
        }
    }

    /// Every field the population names outside `required_when`, as written: the fields of
    /// `entity`, then its time fields (POP-4.3).
    #[must_use]
    pub fn named_fields(&self) -> Vec<&str> {
        let mut fields: Vec<&str> = self.entity.iter().map(String::as_str).collect();
        fields.extend(self.time_fields());
        fields
    }
}

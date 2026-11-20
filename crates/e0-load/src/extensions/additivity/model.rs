//! An additivity declaration of the `additivity` extension (ADD-3), as data types only.

use std::num::NonZeroU64;

use menura_contracts::types::common::Grain;

use crate::names::Name;
use crate::pointer::JsonPointer;
use crate::read::Location;

/// The `additivity` of one metric whose keys and values hold in E0.c (ADD-5.1.1 to
/// ADD-5.1.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdditivityDeclaration {
    /// The JSON Pointer of the metric that declares it.
    pub metric: JsonPointer,
    /// The JSON Pointer of the `data` member of the metric's entry of ours, where every
    /// diagnostic about the declaration is located (E0C-2.6).
    pub data: JsonPointer,
    /// Where that `data` member starts in the document text.
    pub location: Location,
    /// What the metric declares.
    pub additivity: DeclaredAdditivity,
}

/// The value of `type`, with the keys of `semi_additive` (ADD-3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclaredAdditivity {
    /// `non_additive`: always computed from base rows at the query grain (ADD-3.1).
    NonAdditive,
    /// `semi_additive`: not summed across one field (ADD-3.2).
    SemiAdditive(SemiAdditive),
}

/// The keys of a `semi_additive` declaration (ADD-3.2).
///
/// The keys that depend on the kind of `over` are kept as written: whether they must be
/// present is checked in E0.f, once `over` is typed (ADD-5.1.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemiAdditive {
    /// The field across which the metric is not summed.
    pub over: FieldReference,
    /// The fields that identify each snapshot entity, in order, without repeats; `over` is
    /// not one of them (`E0C-15`).
    pub entity: Vec<FieldReference>,
    /// How the values are combined across `over`, with `selection` for `first` and `last`.
    pub aggregation: SnapshotAggregation,
    /// What a NULL value means.
    pub nulls: NullHandling,
    /// The grain of the snapshots, when given.
    pub snapshot_grain: Option<Grain>,
    /// The name of the expected population, as written, when given.
    pub population: Option<String>,
    /// What an expected entity without a row contributes, when given.
    pub missing: Option<MissingHandling>,
    /// What a row of an entity the population did not expect does, when given.
    pub unexpected: Option<UnexpectedHandling>,
}

/// A qualified field reference `dataset.field` (ADD-5.1.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldReference {
    /// The reference as written, for messages.
    pub written: String,
    /// The dataset, after normalization (E0D-1.1).
    pub dataset: Name,
    /// The field, after normalization (E0D-1.1).
    pub field: Name,
}

impl FieldReference {
    /// Whether `self` and `other` name the same field (E0D-1.3).
    #[must_use]
    pub fn same_field(&self, other: &Self) -> bool {
        self.dataset == other.dataset && self.field == other.field
    }
}

/// The values of `aggregation` (ADD-3.2), with `selection` where it is required.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SnapshotAggregation {
    /// `last`, with its `selection`.
    Last(Selection),
    /// `first`, with its `selection`.
    First(Selection),
    /// `min`.
    Minimum,
    /// `max`.
    Maximum,
    /// `avg`.
    Average,
}

impl SnapshotAggregation {
    /// Whether the aggregation needs time order: `first` and `last` (ADD-5.1.5).
    #[must_use]
    pub fn needs_time_order(self) -> bool {
        matches!(self, Self::Last(_) | Self::First(_))
    }
}

/// The values of `selection` (ADD-3.2, ADD-4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Selection {
    /// `global_period`: the same period for every entity.
    GlobalPeriod,
    /// `per_entity`: each entity's own row.
    PerEntity,
}

/// The values of `nulls` (ADD-3.2, ADD-4.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NullHandling {
    /// `as_missing`: the row is treated as missing; only with a time `over`.
    AsMissing,
    /// `keep`.
    Keep,
    /// `zero`.
    Zero,
    /// `error`.
    Error,
}

/// The values of `missing` (ADD-3.2, ADD-4.3), with the keys of `carry_forward`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MissingHandling {
    /// `exclude`.
    Exclude,
    /// `carry_forward`, with `max_carry_forward` and `on_stale`.
    CarryForward {
        /// The oldest value carried, in `snapshot_grain` periods.
        max_carry_forward: NonZeroU64,
        /// What a value older than that does.
        on_stale: StaleHandling,
    },
    /// `zero`.
    Zero,
    /// `error`.
    Error,
}

/// The values of `on_stale` (ADD-3.2, ADD-4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StaleHandling {
    /// `exclude`.
    Exclude,
    /// `error`.
    Error,
}

/// The values of `unexpected` (ADD-3.2, ADD-4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnexpectedHandling {
    /// `include`.
    Include,
    /// `exclude`.
    Exclude,
    /// `error`.
    Error,
}

impl DeclaredAdditivity {
    /// The `semi_additive` keys, when the declaration is semi-additive.
    #[must_use]
    pub fn semi_additive(&self) -> Option<&SemiAdditive> {
        match self {
            Self::NonAdditive => None,
            Self::SemiAdditive(keys) => Some(keys),
        }
    }
}

//! The listed values of the keys of an additivity declaration (ADD-3).

use menura_contracts::types::common::Grain;

use super::model::{NullHandling, Selection, StaleHandling, UnexpectedHandling};

/// The values of `type`, before their keys are read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TypeName {
    NonAdditive,
    SemiAdditive,
}

/// The values of `aggregation`, before `selection` is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AggregationName {
    Last,
    First,
    Minimum,
    Maximum,
    Average,
}

/// The values of `missing`, before `max_carry_forward` and `on_stale` are read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MissingName {
    Exclude,
    CarryForward,
    Zero,
    Error,
}

pub(super) const TYPES: [(&str, TypeName); 2] = [
    ("non_additive", TypeName::NonAdditive),
    ("semi_additive", TypeName::SemiAdditive),
];

pub(super) const AGGREGATIONS: [(&str, AggregationName); 5] = [
    ("last", AggregationName::Last),
    ("first", AggregationName::First),
    ("min", AggregationName::Minimum),
    ("max", AggregationName::Maximum),
    ("avg", AggregationName::Average),
];

pub(super) const NULL_HANDLINGS: [(&str, NullHandling); 4] = [
    ("as_missing", NullHandling::AsMissing),
    ("keep", NullHandling::Keep),
    ("zero", NullHandling::Zero),
    ("error", NullHandling::Error),
];

/// The values of a grain, shared with the `grain` of a context modifier (CTX-5.1.1).
pub(crate) const GRAINS: [(&str, Grain); 5] = [
    ("day", Grain::Day),
    ("week", Grain::Week),
    ("month", Grain::Month),
    ("quarter", Grain::Quarter),
    ("year", Grain::Year),
];

pub(super) const MISSING_HANDLINGS: [(&str, MissingName); 4] = [
    ("exclude", MissingName::Exclude),
    ("carry_forward", MissingName::CarryForward),
    ("zero", MissingName::Zero),
    ("error", MissingName::Error),
];

pub(super) const UNEXPECTED_HANDLINGS: [(&str, UnexpectedHandling); 3] = [
    ("include", UnexpectedHandling::Include),
    ("exclude", UnexpectedHandling::Exclude),
    ("error", UnexpectedHandling::Error),
];

pub(super) const SELECTIONS: [(&str, Selection); 2] = [
    ("global_period", Selection::GlobalPeriod),
    ("per_entity", Selection::PerEntity),
];

pub(super) const STALE_HANDLINGS: [(&str, StaleHandling); 2] = [
    ("exclude", StaleHandling::Exclude),
    ("error", StaleHandling::Error),
];

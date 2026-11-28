//! The interpreted extension declarations of E0.c, in the form of the internal model
//! (E0H-1.2): additivity, derived and context metrics, and populations.

use menura_model::internal_model::{
    ContextModifier, DeclaredAdditivity, DivisionByZero, Field, FieldPath, MissingHandling,
    NullHandling, OpenEnd, PartialPeriod, Population, PopulationMode, RemovedDimensions, Selection,
    SemiAdditive, SnapshotAggregation, StaleHandling, UnexpectedHandling, ValueType,
};

use super::located::{located, normalized, written};
use super::trees::parse;
use crate::expressions::ExpressionPlace;
use crate::extensions::additivity::{
    self, DeclaredAdditivity as Declared, FieldReference, MissingHandling as Missing,
    NullHandling as Nulls, Selection as Selected, SnapshotAggregation as Aggregation,
    StaleHandling as Stale, UnexpectedHandling as Unexpected,
};
use crate::extensions::context::{Modifier, Removed};
use crate::extensions::derived::DivisionByZero as Division;
use crate::extensions::populations::{
    PartialPeriod as Partial, PopulationMode as Mode, parse_open_end,
};

/// The model's form of `dataset.field`.
pub(super) fn field_path(reference: &FieldReference) -> FieldPath {
    FieldPath {
        dataset: normalized(&reference.dataset),
        field: normalized(&reference.field),
    }
}

/// The model's form of an additivity declaration.
pub(super) fn declared_additivity(declared: &Declared) -> DeclaredAdditivity {
    match declared {
        Declared::NonAdditive => DeclaredAdditivity::NonAdditive,
        Declared::SemiAdditive(keys) => DeclaredAdditivity::SemiAdditive(semi_additive(keys)),
    }
}

fn semi_additive(keys: &additivity::SemiAdditive) -> SemiAdditive {
    SemiAdditive {
        over: field_path(&keys.over),
        entity: keys.entity.iter().map(field_path).collect(),
        aggregation: aggregation(keys.aggregation),
        nulls: match keys.nulls {
            Nulls::AsMissing => NullHandling::AsMissing,
            Nulls::Keep => NullHandling::Keep,
            Nulls::Zero => NullHandling::Zero,
            Nulls::Error => NullHandling::Error,
        },
        snapshot_grain: keys.snapshot_grain,
        population: keys.population.as_deref().map(written),
        missing: keys.missing.map(missing),
        unexpected: keys.unexpected.map(|unexpected| match unexpected {
            Unexpected::Include => UnexpectedHandling::Include,
            Unexpected::Exclude => UnexpectedHandling::Exclude,
            Unexpected::Error => UnexpectedHandling::Error,
        }),
    }
}

fn aggregation(aggregation: Aggregation) -> SnapshotAggregation {
    let selection = |selected: Selected| match selected {
        Selected::GlobalPeriod => Selection::GlobalPeriod,
        Selected::PerEntity => Selection::PerEntity,
    };
    match aggregation {
        Aggregation::Last(selected) => SnapshotAggregation::Last(selection(selected)),
        Aggregation::First(selected) => SnapshotAggregation::First(selection(selected)),
        Aggregation::Minimum => SnapshotAggregation::Minimum,
        Aggregation::Maximum => SnapshotAggregation::Maximum,
        Aggregation::Average => SnapshotAggregation::Average,
    }
}

fn missing(missing: Missing) -> MissingHandling {
    match missing {
        Missing::Exclude => MissingHandling::Exclude,
        Missing::CarryForward {
            max_carry_forward,
            on_stale,
        } => MissingHandling::CarryForward {
            max_carry_forward,
            on_stale: match on_stale {
                Stale::Exclude => StaleHandling::Exclude,
                Stale::Error => StaleHandling::Error,
            },
        },
        Missing::Zero => MissingHandling::Zero,
        Missing::Error => MissingHandling::Error,
    }
}

/// The model's form of `on_division_by_zero`.
pub(super) fn division_by_zero(division: Division) -> DivisionByZero {
    match division {
        Division::Null => DivisionByZero::Null,
        Division::Error => DivisionByZero::Error,
    }
}

/// The model's form of a context modifier.
pub(super) fn context_modifier(modifier: &Modifier) -> ContextModifier {
    match modifier {
        Modifier::PeriodToDate(time) => ContextModifier::PeriodToDate {
            time_dimension: field_path(&time.time_dimension),
            grain: time.grain,
        },
        Modifier::PriorPeriod { time, offset } => ContextModifier::PriorPeriod {
            time_dimension: field_path(&time.time_dimension),
            grain: time.grain,
            offset: *offset,
        },
        Modifier::ShareOfTotal(Removed::All) => {
            ContextModifier::ShareOfTotal(RemovedDimensions::All)
        }
        Modifier::ShareOfTotal(Removed::Fields(fields)) => ContextModifier::ShareOfTotal(
            RemovedDimensions::Fields(fields.iter().map(field_path).collect()),
        ),
    }
}

/// The model's form of a population, with its `required_when` parsed and its `open_end` read
/// as a value of the type of its `valid_to` among `fields`, the fields of its dataset.
pub(super) fn population(
    population: &crate::extensions::Population,
    fields: &[Field],
) -> Population {
    Population {
        name: written(&population.name),
        entity: population
            .entity
            .iter()
            .map(|field| written(field))
            .collect(),
        mode: match &population.mode {
            Mode::Validity {
                valid_from,
                valid_to,
                partial_period,
                valid_to_bound,
                open_end,
            } => PopulationMode::Validity {
                valid_from: written(valid_from),
                valid_to: written(valid_to),
                partial_period: match partial_period {
                    Partial::AnyDay => PartialPeriod::AnyDay,
                    Partial::FullPeriod => PartialPeriod::FullPeriod,
                    Partial::PeriodEnd => PartialPeriod::PeriodEnd,
                },
                valid_to_bound: *valid_to_bound,
                open_end: open_end
                    .as_deref()
                    .and_then(|text| typed_open_end(text, valid_to, fields)),
            },
            Mode::Periodic { period, grain } => PopulationMode::Periodic {
                period: written(period),
                grain: *grain,
            },
        },
        required_when: population
            .required_when
            .as_deref()
            .and_then(|text| parse(text, ExpressionPlace::Extension).ok()),
        location: located(&population.data, population.location),
    }
}

/// `text`, the `open_end` of a population, as a value of the type of its field `valid_to`
/// among `fields`. E0.f rejected every other text (VAL-1.5); `None` only when `valid_to` has
/// no type, which makes the field unusable (E0-4).
fn typed_open_end(text: &str, valid_to: &str, fields: &[Field]) -> Option<OpenEnd> {
    let name = written(valid_to);
    let field = fields.iter().find(|field| field.name == name)?;
    match &field.typed.as_ref()?.value_type {
        ValueType::Logical(logical_type) => parse_open_end(text, logical_type),
        ValueType::Null => None,
    }
}

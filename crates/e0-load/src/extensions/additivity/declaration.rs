//! Reading one `additivity` value: its keys (ADD-5.1.1), their values (ADD-5.1.2, ADD-5.1.3)
//! and the pairs of keys checked in E0.c (ADD-5.1.4, `E0C-15`).

use std::num::NonZeroU64;

use serde_json::{Map, Value as Json};

use super::model::{
    DeclaredAdditivity, FieldReference, MissingHandling, SemiAdditive, SnapshotAggregation,
};
use super::reference::field_reference;
use super::values::{
    AGGREGATIONS, AggregationName, GRAINS, MISSING_HANDLINGS, MissingName, NULL_HANDLINGS,
    SELECTIONS, STALE_HANDLINGS, TYPES, TypeName, UNEXPECTED_HANDLINGS,
};
use crate::extensions::diagnostic::ExtensionCode;

const TYPE: &str = "type";
const OVER: &str = "over";
const ENTITY: &str = "entity";
const AGGREGATION: &str = "aggregation";
const NULLS: &str = "nulls";
const SNAPSHOT_GRAIN: &str = "snapshot_grain";
const POPULATION: &str = "population";
const MISSING: &str = "missing";
const UNEXPECTED: &str = "unexpected";
const SELECTION: &str = "selection";
const MAX_CARRY_FORWARD: &str = "max_carry_forward";
const ON_STALE: &str = "on_stale";

/// The keys of `semi_additive` (ADD-3.2), after `type`.
const SEMI_ADDITIVE_KEYS: [&str; 11] = [
    OVER,
    ENTITY,
    AGGREGATION,
    NULLS,
    SNAPSHOT_GRAIN,
    POPULATION,
    MISSING,
    UNEXPECTED,
    SELECTION,
    MAX_CARRY_FORWARD,
    ON_STALE,
];

/// The keys every `semi_additive` declaration has (ADD-3.2, ADD-5.1.1).
const REQUIRED_KEYS: [&str; 4] = [OVER, ENTITY, AGGREGATION, NULLS];

/// The problems of one declaration: each code with a description in English.
pub(super) type Problems = Vec<(ExtensionCode, String)>;

/// What reading one `additivity` value found.
#[derive(Debug)]
pub(super) struct Reading {
    /// The declaration, when it has no problem.
    pub(super) additivity: Option<DeclaredAdditivity>,
    /// Its problems.
    pub(super) problems: Problems,
}

/// Reads one `additivity` value.
///
/// Without a valid `type`, only `type` and the keys that no type defines (`E0C-02`, E0C-2.4)
/// are checked: the other rules depend on the type.
pub(super) fn read_declaration(value: &Json) -> Reading {
    let mut problems = Problems::new();
    let additivity = read_object(value, &mut problems);
    Reading {
        additivity: additivity.filter(|_| problems.is_empty()),
        problems,
    }
}

fn read_object(value: &Json, problems: &mut Problems) -> Option<DeclaredAdditivity> {
    let Json::Object(members) = value else {
        let message = format!("`additivity` is {value}, not an object");
        problems.push((ExtensionCode::WrongValue, message));
        return None;
    };
    let Some(type_value) = members.get(TYPE) else {
        undefined_keys(members, problems);
        let message = "`additivity` has no `type`".to_owned();
        problems.push((ExtensionCode::MissingAdditivityKey, message));
        return None;
    };
    let mut reader = Reader { problems };
    match reader.choice(TYPE, type_value, &TYPES) {
        None => {
            undefined_keys(members, reader.problems);
            None
        }
        Some(TypeName::NonAdditive) => {
            non_additive_keys(members, reader.problems);
            Some(DeclaredAdditivity::NonAdditive)
        }
        Some(TypeName::SemiAdditive) => {
            undefined_keys(members, reader.problems);
            reader
                .semi_additive(members)
                .map(DeclaredAdditivity::SemiAdditive)
        }
    }
}

/// `E0C-02` for every key that ADD-3 does not define.
fn undefined_keys(members: &Map<String, Json>, problems: &mut Problems) {
    for key in members.keys() {
        if key != TYPE && !SEMI_ADDITIVE_KEYS.contains(&key.as_str()) {
            let message = format!("no additivity type defines the key `{key}`");
            problems.push((ExtensionCode::UnknownKey, message));
        }
    }
}

/// `non_additive` admits no other key, also one that ADD-3.2 defines (`E0C-10`).
fn non_additive_keys(members: &Map<String, Json>, problems: &mut Problems) {
    for key in members.keys().filter(|key| *key != TYPE) {
        let message = format!("`non_additive` admits no other key, and `{key}` is given");
        problems.push((ExtensionCode::NonAdditiveKey, message));
    }
}

/// Reads the keys of one declaration and records their problems.
struct Reader<'problems> {
    problems: &'problems mut Problems,
}

impl Reader<'_> {
    fn semi_additive(&mut self, members: &Map<String, Json>) -> Option<SemiAdditive> {
        for key in REQUIRED_KEYS {
            if !members.contains_key(key) {
                let message = format!("`semi_additive` requires `{key}`");
                self.problems
                    .push((ExtensionCode::MissingAdditivityKey, message));
            }
        }
        let over = members
            .get(OVER)
            .and_then(|value| self.reference(OVER, value));
        let entity = members.get(ENTITY).and_then(|value| self.entity(value));
        let aggregation = self.aggregation(members);
        let nulls = self.optional(members, NULLS, &NULL_HANDLINGS);
        let snapshot_grain = self.optional(members, SNAPSHOT_GRAIN, &GRAINS);
        let population = members
            .get(POPULATION)
            .and_then(|value| self.text(POPULATION, value));
        let missing = self.missing(members);
        let unexpected = self.optional(members, UNEXPECTED, &UNEXPECTED_HANDLINGS);
        if let (Some(over), Some(entity)) = (&over, &entity) {
            self.over_outside_entity(over, entity);
        }
        Some(SemiAdditive {
            over: over?,
            entity: entity?,
            aggregation: aggregation?,
            nulls: nulls?,
            snapshot_grain,
            population,
            missing,
            unexpected,
        })
    }

    /// `aggregation`, with `selection` exactly when it is `first` or `last` (`E0C-14`).
    fn aggregation(&mut self, members: &Map<String, Json>) -> Option<SnapshotAggregation> {
        let aggregation = self.optional(members, AGGREGATION, &AGGREGATIONS);
        let selection = self.optional(members, SELECTION, &SELECTIONS);
        let name = aggregation?;
        let has_selection = members.contains_key(SELECTION);
        let ordered = matches!(name, AggregationName::Last | AggregationName::First);
        if ordered != has_selection {
            let message = if ordered {
                "`selection` is required when `aggregation` is `first` or `last`"
            } else {
                "`selection` is allowed only when `aggregation` is `first` or `last`"
            };
            self.problems
                .push((ExtensionCode::SelectionPair, message.to_owned()));
            return None;
        }
        Some(match name {
            AggregationName::Last => SnapshotAggregation::Last(selection?),
            AggregationName::First => SnapshotAggregation::First(selection?),
            AggregationName::Minimum => SnapshotAggregation::Minimum,
            AggregationName::Maximum => SnapshotAggregation::Maximum,
            AggregationName::Average => SnapshotAggregation::Average,
        })
    }

    /// `missing`, with `max_carry_forward` and `on_stale` exactly when it is `carry_forward`,
    /// and `max_carry_forward` greater than 0 (`E0C-13`). `None` when `missing` is absent or
    /// has a problem.
    fn missing(&mut self, members: &Map<String, Json>) -> Option<MissingHandling> {
        let name = self.optional(members, MISSING, &MISSING_HANDLINGS);
        let limit = self.max_carry_forward(members);
        let on_stale = self.optional(members, ON_STALE, &STALE_HANDLINGS);
        if members.contains_key(MISSING) && name.is_none() {
            return None;
        }
        let carry_forward = name == Some(MissingName::CarryForward);
        for key in [MAX_CARRY_FORWARD, ON_STALE] {
            if members.contains_key(key) != carry_forward {
                let message = if carry_forward {
                    format!("`{key}` is required when `missing` is `carry_forward`")
                } else {
                    format!("`{key}` is allowed only when `missing` is `carry_forward`")
                };
                self.problems
                    .push((ExtensionCode::CarryForwardKeys, message));
            }
        }
        Some(match name? {
            MissingName::Exclude => MissingHandling::Exclude,
            MissingName::Zero => MissingHandling::Zero,
            MissingName::Error => MissingHandling::Error,
            MissingName::CarryForward => MissingHandling::CarryForward {
                max_carry_forward: limit?,
                on_stale: on_stale?,
            },
        })
    }

    /// `max_carry_forward`: an integer (`E0C-03`) greater than 0 (`E0C-13`); `None` when the
    /// key is absent or has a problem.
    fn max_carry_forward(&mut self, members: &Map<String, Json>) -> Option<NonZeroU64> {
        let value = members.get(MAX_CARRY_FORWARD)?;
        let Json::Number(number) = value else {
            return self.wrong_value(MAX_CARRY_FORWARD, value, "an integer");
        };
        if number.as_u64().is_none() && number.as_i64().is_none() {
            return self.wrong_value(MAX_CARRY_FORWARD, value, "an integer");
        }
        let limit = number.as_u64().and_then(NonZeroU64::new);
        if limit.is_none() {
            let message = format!("`max_carry_forward` is {number}, not greater than 0");
            self.problems
                .push((ExtensionCode::CarryForwardKeys, message));
        }
        limit
    }

    /// `E0C-15`: `over` is not one of the fields of `entity` (ADD-5.1.3).
    fn over_outside_entity(&mut self, over: &FieldReference, entity: &[FieldReference]) {
        if let Some(found) = entity.iter().find(|field| field.same_field(over)) {
            let message = format!(
                "`over` `{}` is part of `entity`, as `{}`",
                over.written, found.written
            );
            self.problems.push((ExtensionCode::OverInEntity, message));
        }
    }

    /// `entity`: a non-empty list of field references without repeats (ADD-5.1.2).
    fn entity(&mut self, value: &Json) -> Option<Vec<FieldReference>> {
        let items = match value {
            Json::Array(items) if !items.is_empty() => items,
            _ => return self.wrong_value(ENTITY, value, "a non-empty list of field references"),
        };
        let mut fields: Vec<FieldReference> = Vec::new();
        for item in items {
            let field = self.reference(ENTITY, item)?;
            if fields.iter().any(|seen| seen.same_field(&field)) {
                let message = format!("`entity` repeats the field `{}`", field.written);
                self.problems.push((ExtensionCode::WrongValue, message));
                return None;
            }
            fields.push(field);
        }
        Some(fields)
    }

    /// A field reference `dataset.field` (ADD-5.1.3).
    fn reference(&mut self, key: &str, value: &Json) -> Option<FieldReference> {
        let reference = value.as_str().and_then(field_reference);
        if reference.is_none() {
            return self.wrong_value(key, value, "a field reference `dataset.field`");
        }
        reference
    }

    fn text(&mut self, key: &str, value: &Json) -> Option<String> {
        match value {
            Json::String(text) => Some(text.clone()),
            _ => self.wrong_value(key, value, "a string"),
        }
    }

    /// The value of the key `key`, one of the strings of `options`, when it is present.
    fn optional<T: Copy>(
        &mut self,
        members: &Map<String, Json>,
        key: &str,
        options: &[(&str, T)],
    ) -> Option<T> {
        members
            .get(key)
            .and_then(|value| self.choice(key, value, options))
    }

    /// A key whose value is one of the strings of `options`.
    fn choice<T: Copy>(&mut self, key: &str, value: &Json, options: &[(&str, T)]) -> Option<T> {
        let found = options
            .iter()
            .find(|(text, _)| value.as_str() == Some(*text))
            .map(|(_, option)| *option);
        if found.is_none() {
            let allowed: Vec<String> = options
                .iter()
                .map(|(text, _)| format!("`{text}`"))
                .collect();
            return self.wrong_value(key, value, &format!("one of {}", allowed.join(", ")));
        }
        found
    }

    fn wrong_value<T>(&mut self, key: &str, value: &Json, expected: &str) -> Option<T> {
        let message = format!("`{key}` is {value}, not {expected}");
        self.problems.push((ExtensionCode::WrongValue, message));
        None
    }
}

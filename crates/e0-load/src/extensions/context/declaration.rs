//! Reading one `context` value: its keys and values (CTX-5.1.1) and its field references
//! (CTX-5.1.2).

use std::num::NonZeroU64;

use menura_contracts::types::common::Grain;
use serde_json::{Map, Value as Json};

use super::model::{Modifier, Removed, TimeModifier};
use crate::extensions::additivity::{FieldReference, GRAINS, field_reference};
use crate::extensions::diagnostic::ExtensionCode;
use crate::extensions::problems::Problems;

const BASE: &str = "base";
const MODIFIER: &str = "modifier";
const TYPE: &str = "type";
const TIME_DIMENSION: &str = "time_dimension";
const GRAIN: &str = "grain";
const OFFSET: &str = "offset";
const REMOVE: &str = "remove";

/// The keys of `context` (CTX-2).
const CONTEXT_KEYS: [&str; 2] = [BASE, MODIFIER];

/// The value of `remove` that takes every dimension out of the grouping (CTX-2).
const ALL: &str = "all";

/// The modifier types of CTX-2, with the keys each lists after `type`.
const TYPES: [(&str, ModifierType, &[&str]); 3] = [
    (
        "period_to_date",
        ModifierType::PeriodToDate,
        &[TIME_DIMENSION, GRAIN],
    ),
    (
        "prior_period",
        ModifierType::PriorPeriod,
        &[TIME_DIMENSION, GRAIN, OFFSET],
    ),
    ("share_of_total", ModifierType::ShareOfTotal, &[REMOVE]),
];

/// The values of `modifier.type`, before their keys are read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModifierType {
    PeriodToDate,
    PriorPeriod,
    ShareOfTotal,
}

/// The value of `base` as written and the modifier of one `context` value, or `None` when
/// `problems` records anything wrong with it.
pub(super) fn read_context(value: &Json, problems: &mut Problems) -> Option<(String, Modifier)> {
    let Json::Object(members) = value else {
        problems.push(wrong_value("`context`", value, "an object"));
        return None;
    };
    for key in members.keys() {
        if !CONTEXT_KEYS.contains(&key.as_str()) {
            let message =
                format!("`context` has no key `{key}`; its keys are `base` and `modifier`");
            problems.push((ExtensionCode::UnknownKey, message));
        }
    }
    let base = required(members, "`context`", BASE, problems).and_then(|value| {
        let text = value.as_str().map(str::to_owned);
        if text.is_none() {
            problems.push(wrong_value("`base`", value, "a string"));
        }
        text
    });
    let modifier = required(members, "`context`", MODIFIER, problems)
        .and_then(|value| read_modifier(value, problems));
    Some((base?, modifier?))
}

/// Reads `modifier`. Without a valid `type`, only `type` and the keys that no type defines
/// (`E0C-02`, E0C-2.4) are checked: the other rules depend on the type.
fn read_modifier(value: &Json, problems: &mut Problems) -> Option<Modifier> {
    let Json::Object(members) = value else {
        problems.push(wrong_value("`modifier`", value, "an object"));
        return None;
    };
    let found = required(members, "`modifier`", TYPE, problems).and_then(|value| {
        let found = TYPES.iter().find(|(name, ..)| value.as_str() == Some(name));
        if found.is_none() {
            problems.push(wrong_value(
                "`type`",
                value,
                "`period_to_date`, `prior_period` or `share_of_total`",
            ));
        }
        found
    });
    let Some((name, modifier_type, keys)) = found else {
        undefined_keys(members, problems);
        return None;
    };
    for key in members.keys() {
        if key != TYPE && !keys.contains(&key.as_str()) {
            let message = format!("the modifier `{name}` has no key `{key}`");
            problems.push((ExtensionCode::UnknownKey, message));
        }
    }
    let mut reader = Reader {
        members,
        name,
        problems,
    };
    match modifier_type {
        ModifierType::PeriodToDate => reader.time().map(Modifier::PeriodToDate),
        ModifierType::PriorPeriod => {
            let time = reader.time();
            let offset = reader.offset();
            Some(Modifier::PriorPeriod {
                time: time?,
                offset: offset?,
            })
        }
        ModifierType::ShareOfTotal => reader.remove().map(Modifier::ShareOfTotal),
    }
}

/// `E0C-02` for every key of `modifier` that no modifier type defines.
fn undefined_keys(members: &Map<String, Json>, problems: &mut Problems) {
    for key in members.keys() {
        let defined = key == TYPE || TYPES.iter().any(|(.., keys)| keys.contains(&key.as_str()));
        if !defined {
            let message = format!("no modifier type defines the key `{key}`");
            problems.push((ExtensionCode::UnknownKey, message));
        }
    }
}

/// The value of the key `key` of `owner`, recording its absence as `E0C-03`.
fn required<'value>(
    members: &'value Map<String, Json>,
    owner: &str,
    key: &str,
    problems: &mut Problems,
) -> Option<&'value Json> {
    let value = members.get(key);
    if value.is_none() {
        problems.push((
            ExtensionCode::WrongValue,
            format!("{owner} requires `{key}`"),
        ));
    }
    value
}

/// The `E0C-03` problem of `what`, whose value `value` is not `expected`.
fn wrong_value(what: &str, value: &Json, expected: &str) -> (ExtensionCode, String) {
    (
        ExtensionCode::WrongValue,
        format!("{what} is {value}, not {expected}"),
    )
}

/// Reads the keys of one modifier of a known type and records their problems.
struct Reader<'members, 'problems> {
    members: &'members Map<String, Json>,
    name: &'static str,
    problems: &'problems mut Problems,
}

impl Reader<'_, '_> {
    /// The value of `key`, recording its absence as `E0C-03`.
    fn required(&mut self, key: &str) -> Option<&Json> {
        let owner = format!("the modifier `{}`", self.name);
        required(self.members, &owner, key, self.problems)
    }

    /// `time_dimension` and `grain`.
    fn time(&mut self) -> Option<TimeModifier> {
        let time_dimension = self
            .required(TIME_DIMENSION)
            .cloned()
            .and_then(|value| self.reference(TIME_DIMENSION, &value));
        let grain = self
            .required(GRAIN)
            .cloned()
            .and_then(|value| self.grain(&value));
        Some(TimeModifier {
            time_dimension: time_dimension?,
            grain: grain?,
        })
    }

    fn grain(&mut self, value: &Json) -> Option<Grain> {
        let found = GRAINS
            .iter()
            .find(|(name, _)| value.as_str() == Some(name))
            .map(|(_, grain)| *grain);
        if found.is_none() {
            let expected = "`day`, `week`, `month`, `quarter` or `year`";
            self.problems.push(wrong_value("`grain`", value, expected));
        }
        found
    }

    /// `offset`: a number (`E0C-03`) that is a positive integer (`E0C-62`).
    fn offset(&mut self) -> Option<NonZeroU64> {
        let value = self.required(OFFSET)?.clone();
        let Json::Number(number) = &value else {
            self.problems
                .push(wrong_value("`offset`", &value, "a number"));
            return None;
        };
        let offset = number.as_u64().and_then(NonZeroU64::new);
        if offset.is_none() {
            let message = format!("`offset` is {number}, not a positive integer");
            self.problems.push((ExtensionCode::ContextOffset, message));
        }
        offset
    }

    /// `remove`: the string `all`, or a non-empty list of field references without repeats.
    fn remove(&mut self) -> Option<Removed> {
        let value = self.required(REMOVE)?.clone();
        let expected = "the string `all` or a non-empty list of fields";
        match &value {
            Json::String(text) if text == ALL => Some(Removed::All),
            Json::Array(items) if !items.is_empty() => self.removed_fields(items),
            _ => {
                self.problems
                    .push(wrong_value("`remove`", &value, expected));
                None
            }
        }
    }

    fn removed_fields(&mut self, items: &[Json]) -> Option<Removed> {
        let mut fields: Vec<FieldReference> = Vec::new();
        let mut valid = true;
        for item in items {
            let Some(field) = self.reference(REMOVE, item) else {
                valid = false;
                continue;
            };
            if let Some(seen) = fields.iter().find(|seen| seen.same_field(&field)) {
                let message = format!(
                    "`remove` lists the field `{}` twice, also as `{}`",
                    seen.written, field.written
                );
                self.problems.push((ExtensionCode::WrongValue, message));
                valid = false;
            } else {
                fields.push(field);
            }
        }
        valid.then_some(Removed::Fields(fields))
    }

    /// A field reference `dataset.field` (CTX-5.1.2), read as ADD-5.1.3 states.
    fn reference(&mut self, key: &str, value: &Json) -> Option<FieldReference> {
        let reference = value.as_str().and_then(field_reference);
        if reference.is_none() {
            let what = if key == REMOVE {
                "a `remove` item".to_owned()
            } else {
                format!("`{key}`")
            };
            self.problems.push(wrong_value(
                &what,
                value,
                "a field reference `dataset.field`",
            ));
        }
        reference
    }
}

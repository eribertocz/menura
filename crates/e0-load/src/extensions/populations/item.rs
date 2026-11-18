//! Reading one item of a `populations` list: its keys (POP-4.1) and their values (POP-4.2).

use semantic_compiler_contracts::types::common::Grain;
use serde_json::{Map, Value as Json};

use super::model::{PartialPeriod, PopulationMode};
use crate::names::Name;

const NAME: &str = "name";
const MODE: &str = "mode";
const ENTITY: &str = "entity";
const REQUIRED_WHEN: &str = "required_when";
const VALID_FROM: &str = "valid_from";
const VALID_TO: &str = "valid_to";
const PARTIAL_PERIOD: &str = "partial_period";
const PERIOD: &str = "period";
const GRAIN: &str = "grain";

/// The keys every mode has (POP-2).
const COMMON_KEYS: [&str; 4] = [NAME, MODE, ENTITY, REQUIRED_WHEN];

/// The keys of the mode `validity` (POP-2).
const VALIDITY_KEYS: [&str; 3] = [VALID_FROM, VALID_TO, PARTIAL_PERIOD];

/// The keys of the mode `periodic` (POP-2).
const PERIODIC_KEYS: [&str; 2] = [PERIOD, GRAIN];

const MODES: [(&str, Mode); 2] = [("validity", Mode::Validity), ("periodic", Mode::Periodic)];

const PARTIAL_PERIODS: [(&str, PartialPeriod); 3] = [
    ("any_day", PartialPeriod::AnyDay),
    ("full_period", PartialPeriod::FullPeriod),
    ("period_end", PartialPeriod::PeriodEnd),
];

const GRAINS: [(&str, Grain); 5] = [
    ("day", Grain::Day),
    ("week", Grain::Week),
    ("month", Grain::Month),
    ("quarter", Grain::Quarter),
    ("year", Grain::Year),
];

/// The two modes, before their keys are read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Validity,
    Periodic,
}

impl Mode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Validity => "validity",
            Self::Periodic => "periodic",
        }
    }

    fn keys(self) -> &'static [&'static str] {
        match self {
            Self::Validity => &VALIDITY_KEYS,
            Self::Periodic => &PERIODIC_KEYS,
        }
    }

    fn other(self) -> Self {
        match self {
            Self::Validity => Self::Periodic,
            Self::Periodic => Self::Validity,
        }
    }
}

/// The problems of one item, described in English, by code.
#[derive(Debug, Default)]
pub(super) struct Problems {
    /// `E0C-02`: keys that POP-2 does not define (POP-4.1).
    pub(super) unknown: Vec<String>,
    /// `E0C-21`: missing keys, and keys of the other mode (POP-4.1).
    pub(super) mode_keys: Vec<String>,
    /// `E0C-03`: values of a wrong type or outside their set (POP-4.2).
    pub(super) values: Vec<String>,
}

impl Problems {
    fn is_empty(&self) -> bool {
        self.unknown.is_empty() && self.mode_keys.is_empty() && self.values.is_empty()
    }
}

/// The keys of an item without problems.
#[derive(Debug)]
pub(super) struct Item {
    pub(super) name: String,
    pub(super) entity: Vec<String>,
    pub(super) mode: PopulationMode,
    pub(super) required_when: Option<String>,
}

/// What reading one item found.
#[derive(Debug)]
pub(super) struct Reading<'json> {
    /// The `name`, when it is a non-empty string, even if the item has other problems: it is
    /// compared across the model all the same (POP-4.3).
    pub(super) name: Option<&'json str>,
    /// The item, when it has no problem.
    pub(super) item: Option<Item>,
    /// Its problems.
    pub(super) problems: Problems,
}

/// Reads the item at `index` of a `populations` list.
pub(super) fn read_item(index: usize, item: &Json) -> Reading<'_> {
    let mut reader = Reader {
        label: format!("item {index}"),
        problems: Problems::default(),
    };
    let Json::Object(members) = item else {
        let message = format!("{} of populations is not an object", reader.label);
        reader.problems.values.push(message);
        return Reading {
            name: None,
            item: None,
            problems: reader.problems,
        };
    };
    reader.unknown_keys(members);
    let name = reader
        .required(members, NAME)
        .and_then(|value| reader.name(value));
    let entity = reader
        .required(members, ENTITY)
        .and_then(|value| reader.entity(value));
    let mode = reader
        .required(members, MODE)
        .and_then(|value| reader.choice(MODE, value, &MODES));
    let required_when = members
        .get(REQUIRED_WHEN)
        .map(|value| reader.text(REQUIRED_WHEN, value));
    let mode_keys = mode.map(|mode| reader.mode_keys(members, mode));
    let item = reader
        .problems
        .is_empty()
        .then(|| {
            Some(Item {
                name: name?.to_owned(),
                entity: entity?,
                mode: mode_keys??,
                required_when: match required_when {
                    None => None,
                    Some(text) => Some(text?.to_owned()),
                },
            })
        })
        .flatten();
    Reading {
        name,
        item,
        problems: reader.problems,
    }
}

/// Reads the keys of one item and records their problems.
struct Reader {
    /// The item, as messages name it, such as `item 0`.
    label: String,
    problems: Problems,
}

impl Reader {
    fn unknown_keys(&mut self, members: &Map<String, Json>) {
        let defined = COMMON_KEYS
            .iter()
            .chain(&VALIDITY_KEYS)
            .chain(&PERIODIC_KEYS);
        for key in members.keys() {
            if !defined.clone().any(|defined| defined == key) {
                let message = format!("`{key}` in {}", self.label);
                self.problems.unknown.push(message);
            }
        }
    }

    /// The value of the key `key`, which every item has; `E0C-21` when it is missing.
    fn required<'json>(
        &mut self,
        members: &'json Map<String, Json>,
        key: &str,
    ) -> Option<&'json Json> {
        let value = members.get(key);
        if value.is_none() {
            let message = format!("{} has no `{key}`", self.label);
            self.problems.mode_keys.push(message);
        }
        value
    }

    /// `name`: a non-empty string.
    fn name<'json>(&mut self, value: &'json Json) -> Option<&'json str> {
        match value {
            Json::String(text) if !text.is_empty() => Some(text),
            _ => self.wrong_value(NAME, value, "a non-empty string"),
        }
    }

    /// `entity`: a non-empty list of field names without repeats (E0D-1.3).
    fn entity(&mut self, value: &Json) -> Option<Vec<String>> {
        let names: Option<Vec<String>> = match value {
            Json::Array(items) if !items.is_empty() => items
                .iter()
                .map(|item| item.as_str().map(str::to_owned))
                .collect(),
            _ => None,
        };
        let Some(names) = names else {
            return self.wrong_value(ENTITY, value, "a non-empty list of field names");
        };
        let mut seen = Vec::new();
        for name in &names {
            let normalized = Name::declared(name);
            if seen.contains(&normalized) {
                let message = format!("{}: `entity` repeats the field `{name}`", self.label);
                self.problems.values.push(message);
                return None;
            }
            seen.push(normalized);
        }
        Some(names)
    }

    /// A string key: a field name or the text of `required_when`.
    fn text<'json>(&mut self, key: &str, value: &'json Json) -> Option<&'json str> {
        match value {
            Json::String(text) => Some(text),
            _ => self.wrong_value(key, value, "a string"),
        }
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
        let message = format!("{}: `{key}` is {value}, not {expected}", self.label);
        self.problems.values.push(message);
        None
    }

    /// The keys of `mode`: `E0C-21` for a missing one and for every key of the other mode, and
    /// `E0C-03` for a value of a wrong type or outside its set.
    fn mode_keys(&mut self, members: &Map<String, Json>, mode: Mode) -> Option<PopulationMode> {
        let other = mode.other();
        for key in other
            .keys()
            .iter()
            .filter(|key| members.contains_key(**key))
        {
            let message = format!(
                "{}: `{key}` is a key of the mode `{}`, not of `{}`",
                self.label,
                other.as_str(),
                mode.as_str()
            );
            self.problems.mode_keys.push(message);
        }
        let field = |reader: &mut Self, key: &str| {
            reader
                .mode_key(members, key, mode)
                .and_then(|value| reader.text(key, value))
                .map(str::to_owned)
        };
        match mode {
            Mode::Validity => {
                let valid_from = field(self, VALID_FROM);
                let valid_to = field(self, VALID_TO);
                let partial_period = self
                    .mode_key(members, PARTIAL_PERIOD, mode)
                    .and_then(|value| self.choice(PARTIAL_PERIOD, value, &PARTIAL_PERIODS));
                Some(PopulationMode::Validity {
                    valid_from: valid_from?,
                    valid_to: valid_to?,
                    partial_period: partial_period?,
                })
            }
            Mode::Periodic => {
                let period = field(self, PERIOD);
                let grain = self
                    .mode_key(members, GRAIN, mode)
                    .and_then(|value| self.choice(GRAIN, value, &GRAINS));
                Some(PopulationMode::Periodic {
                    period: period?,
                    grain: grain?,
                })
            }
        }
    }

    /// The value of the key `key` of `mode`; `E0C-21` when it is missing.
    fn mode_key<'json>(
        &mut self,
        members: &'json Map<String, Json>,
        key: &str,
        mode: Mode,
    ) -> Option<&'json Json> {
        let value = members.get(key);
        if value.is_none() {
            let message = format!(
                "{} has no `{key}`, which the mode `{}` requires",
                self.label,
                mode.as_str()
            );
            self.problems.mode_keys.push(message);
        }
        value
    }
}

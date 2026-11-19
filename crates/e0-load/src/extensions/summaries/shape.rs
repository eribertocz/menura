//! Reading one `aggregate` value (SUM-6.1.1): `E0C-02`, `E0C-03`, and `E0C-50` for a missing
//! `of`.

use serde_json::{Map, Value as Json};

use super::model::{Grain, SummaryDimension, SummaryMeasure};
use crate::extensions::diagnostic::ExtensionCode;
use crate::extensions::problems::Problems;
use crate::extensions::security::values::{object, required, text, wrong_value};

const OF: &str = "of";
const DIMENSIONS: &str = "dimensions";
const MEASURES: &str = "measures";
const COVERAGE_FIELD: &str = "coverage_field";
const FIELD: &str = "field";
const BASE: &str = "base";
const GRAIN: &str = "grain";
const BASE_AGGREGATE: &str = "base_aggregate";

/// The parts of an `aggregate` whose shape is valid.
pub(super) struct AggregateShape {
    pub(super) of: String,
    pub(super) dimensions: Vec<SummaryDimension>,
    pub(super) measures: Vec<SummaryMeasure>,
    pub(super) coverage_field: String,
}

/// Reads the `aggregate` of a dataset.
///
/// Returns its parts when `problems` records nothing wrong with it.
pub(super) fn read_aggregate(value: &Json, problems: &mut Problems) -> Option<AggregateShape> {
    let before = problems.len();
    let keys = [OF, DIMENSIONS, MEASURES, COVERAGE_FIELD];
    let members = object(value, "`aggregate`", &keys, problems)?;
    let of = read_of(members, problems);
    let dimensions = required(members, "`aggregate`", DIMENSIONS, problems)
        .and_then(|value| read_list(value, "`aggregate.dimensions`", read_dimension, problems));
    let measures = required(members, "`aggregate`", MEASURES, problems)
        .and_then(|value| read_list(value, "`aggregate.measures`", read_measure, problems));
    let coverage_field = required(members, "`aggregate`", COVERAGE_FIELD, problems)
        .and_then(|value| text(value, "`aggregate.coverage_field`", problems));
    let shape = AggregateShape {
        of: of?.to_owned(),
        dimensions: dimensions?,
        measures: measures?,
        coverage_field: coverage_field?.to_owned(),
    };
    (problems.len() == before).then_some(shape)
}

/// `of`: `E0C-50` when it is missing, `E0C-03` when it is not a string.
fn read_of<'json>(
    members: &'json Map<String, Json>,
    problems: &mut Problems,
) -> Option<&'json str> {
    let Some(value) = members.get(OF) else {
        problems.push((
            ExtensionCode::SummaryBase,
            "`aggregate` requires `of`".to_owned(),
        ));
        return None;
    };
    text(value, "`aggregate.of`", problems)
}

/// The items of `value`, a non-empty list read item by item with `read_item`; `E0C-03` when
/// it is not a non-empty list.
fn read_list<Item>(
    value: &Json,
    what: &str,
    read_item: fn(&Json, &str, &mut Problems) -> Option<Item>,
    problems: &mut Problems,
) -> Option<Vec<Item>> {
    let items = match value.as_array() {
        Some(items) if !items.is_empty() => items,
        _ => {
            problems.push(wrong_value(what, value, "a non-empty list of objects"));
            return None;
        }
    };
    let read: Vec<Option<Item>> = items
        .iter()
        .enumerate()
        .map(|(index, item)| read_item(item, &format!("item {index} of {what}"), problems))
        .collect();
    read.into_iter().collect()
}

/// One item of `dimensions`: an object with `field` and `base`, strings, and an optional
/// `grain` of SUM-6.1.1.
fn read_dimension(value: &Json, what: &str, problems: &mut Problems) -> Option<SummaryDimension> {
    let members = object(value, what, &[FIELD, BASE, GRAIN], problems)?;
    let field = read_text(members, what, FIELD, problems);
    let base = read_text(members, what, BASE, problems);
    let grain = match members.get(GRAIN) {
        None => Some(None),
        Some(value) => read_grain(value, &format!("the `grain` of {what}"), problems).map(Some),
    };
    Some(SummaryDimension {
        field: field?,
        base: base?,
        grain: grain?,
    })
}

/// One item of `measures`: an object with exactly `field` and `base_aggregate`, strings.
fn read_measure(value: &Json, what: &str, problems: &mut Problems) -> Option<SummaryMeasure> {
    let members = object(value, what, &[FIELD, BASE_AGGREGATE], problems)?;
    let field = read_text(members, what, FIELD, problems);
    let base_aggregate = read_text(members, what, BASE_AGGREGATE, problems);
    Some(SummaryMeasure {
        field: field?,
        base_aggregate: base_aggregate?,
    })
}

/// The required string `key` of `members`, the object `what`.
fn read_text(
    members: &Map<String, Json>,
    what: &str,
    key: &str,
    problems: &mut Problems,
) -> Option<String> {
    let value = required(members, what, key, problems)?;
    text(value, &format!("the `{key}` of {what}"), problems).map(str::to_owned)
}

/// A grain of SUM-6.1.1, written exactly; `E0C-03` otherwise.
fn read_grain(value: &Json, what: &str, problems: &mut Problems) -> Option<Grain> {
    let found = Grain::ALL
        .iter()
        .find(|(name, _)| value.as_str() == Some(name))
        .map(|(_, grain)| *grain);
    if found.is_none() {
        problems.push(wrong_value(
            what,
            value,
            "`day`, `week`, `month`, `quarter` or `year`",
        ));
    }
    found
}

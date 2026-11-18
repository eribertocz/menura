//! The `populations` extension, in E0.c: the shape and keys of every population (POP-4.1,
//! POP-4.2) and the uniqueness of population names across the model (POP-4.3).
//!
//! The fields a population names are resolved in E0.d (`E0C-22`) and its time fields and
//! condition are typed in E0.f (`E0C-23`, `E0C-24`), each against the [`Population`] read here.

#[cfg(test)]
pub(crate) mod fixture;
mod item;
mod model;

pub(crate) use model::Population;

use serde_json::Value as Json;

use super::diagnostic::{ExtensionCode, ExtensionDiagnostic};
use super::output::OwnEntry;
use crate::names::Name;
use item::{Problems, read_item};

/// The extension key of populations (MDL-2).
const POPULATIONS: &str = "populations";

/// The populations of a model and the E0.c diagnostics about them.
#[derive(Debug, Default)]
pub(crate) struct Populations {
    /// Every population without problems, in document order.
    pub(crate) populations: Vec<Population>,
    /// `E0C-02`, `E0C-03`, `E0C-20` and `E0C-21`, at most one of each code per entry, in
    /// document order then by code.
    pub(crate) errors: Vec<ExtensionDiagnostic>,
}

/// Reads the `populations` of every entry of `entries`, which are in document order.
pub(crate) fn validate_populations(entries: &[OwnEntry]) -> Populations {
    let mut found = Populations::default();
    let mut names: Vec<Name> = Vec::new();
    for entry in entries {
        if let Some(value) = entry.extensions.get(POPULATIONS) {
            read_entry(entry, value, &mut names, &mut found);
        }
    }
    found
}

/// Reads the `populations` value of `entry`, comparing each name with `names`, the population
/// names seen so far in the model (POP-4.3).
fn read_entry(entry: &OwnEntry, value: &Json, names: &mut Vec<Name>, found: &mut Populations) {
    let mut problems = Problems::default();
    let mut duplicates = Vec::new();
    for (index, item) in list(value, &mut problems).iter().enumerate() {
        let reading = read_item(index, item);
        if let Some(name) = reading.name {
            let normalized = Name::declared(name);
            if names.contains(&normalized) {
                duplicates.push(format!("`{name}` (item {index})"));
            } else {
                names.push(normalized);
            }
        }
        merge(&mut problems, reading.problems);
        if let Some(item) = reading.item {
            found.populations.push(Population {
                dataset: entry.object_pointer.clone(),
                data: entry.data_pointer.clone(),
                location: entry.location,
                name: item.name,
                entity: item.entity,
                mode: item.mode,
                required_when: item.required_when,
            });
        }
    }
    report(entry, problems, duplicates, &mut found.errors);
}

/// At most one diagnostic per code for `entry`, in the order of the codes.
fn report(
    entry: &OwnEntry,
    problems: Problems,
    duplicates: Vec<String>,
    errors: &mut Vec<ExtensionDiagnostic>,
) {
    let report = [
        (
            ExtensionCode::UnknownKey,
            "no population defines the key",
            problems.unknown,
        ),
        (
            ExtensionCode::WrongValue,
            "wrong population value",
            problems.values,
        ),
        (
            ExtensionCode::DuplicatePopulation,
            "the name of an earlier population of the model is repeated by",
            duplicates,
        ),
        (
            ExtensionCode::PopulationModeKeys,
            "population keys do not match the mode",
            problems.mode_keys,
        ),
    ];
    for (code, summary, messages) in report {
        if !messages.is_empty() {
            let message = format!("{summary}: {}", messages.join("; "));
            errors.push(entry_diagnostic(entry, code, message));
        }
    }
}

/// The items of the `populations` value; `E0C-03` and no items when it is not a list.
fn list<'json>(value: &'json Json, problems: &mut Problems) -> &'json [Json] {
    if let Json::Array(items) = value {
        return items;
    }
    problems
        .values
        .push(format!("`populations` is {value}, not a list of objects"));
    &[]
}

fn merge(problems: &mut Problems, found: Problems) {
    problems.unknown.extend(found.unknown);
    problems.mode_keys.extend(found.mode_keys);
    problems.values.extend(found.values);
}

/// A diagnostic at the `data` member of `entry` (E0C-2.6).
fn entry_diagnostic(entry: &OwnEntry, code: ExtensionCode, message: String) -> ExtensionDiagnostic {
    ExtensionDiagnostic {
        code,
        pointer: entry.data_pointer.clone(),
        location: entry.location,
        message,
    }
}

/// One diagnostic with `code` per entry of `populations` for which `problems` finds anything,
/// joining the problems of its populations; at the entry's `data` member (E0C-2.6, POP-4.6).
///
/// `populations` are in document order, so the populations of one entry are consecutive and
/// the diagnostics are in document order.
pub(crate) fn per_entry<'population>(
    populations: impl IntoIterator<Item = &'population Population>,
    code: ExtensionCode,
    summary: &str,
    mut problems: impl FnMut(&'population Population) -> Vec<String>,
) -> Vec<ExtensionDiagnostic> {
    let mut groups: Vec<(&Population, Vec<String>)> = Vec::new();
    for population in populations {
        let found = problems(population);
        match groups.last_mut() {
            Some((first, messages)) if first.data == population.data => messages.extend(found),
            _ => groups.push((population, found)),
        }
    }
    groups
        .into_iter()
        .filter_map(|(first, messages)| population_diagnostic(first, code, summary, &messages))
        .collect()
}

fn population_diagnostic(
    population: &Population,
    code: ExtensionCode,
    summary: &str,
    messages: &[String],
) -> Option<ExtensionDiagnostic> {
    (!messages.is_empty()).then(|| ExtensionDiagnostic {
        code,
        pointer: population.data.clone(),
        location: population.location,
        message: format!("{summary}: {}", messages.join("; ")),
    })
}

#[cfg(test)]
mod tests;

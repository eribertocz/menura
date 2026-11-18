//! `E0C-12` and `E0C-17`: the options of a semi-additive declaration match the kind of `over`
//! (ADD-5.1.5), and its population matches its `entity` and `snapshot_grain` (ADD-5.1.7),
//! against the field types E0.f found.
//!
//! `E0C-12` is checked only when E0.f typed the `over` field, and an `entity` pair of
//! `E0C-17` only when E0.f typed both fields: a field without a type already has its own
//! diagnostic.

use std::collections::HashMap;

use semantic_compiler_contracts::types::common::LogicalType;

use super::keys::same_kind;
use super::output::{ElementKind, Typing};
use super::value_type::{Typed, describe};
use crate::extensions::additivity::{
    AdditivityDeclaration, FieldReference, NullHandling, SemiAdditive,
};
use crate::extensions::populations::PopulationMode;
use crate::extensions::{ExtensionCode, ExtensionDiagnostic, Extensions, Population};
use crate::names::Name;
use crate::outline::{DatasetOutline, outline};
use crate::pointer::JsonPointer;
use crate::structure::Structure;

/// The datasets and field types the checks of additivity read.
struct Fields<'model, 'document> {
    by_name: HashMap<&'model Name, &'model DatasetOutline<'document>>,
    by_pointer: HashMap<&'model JsonPointer, &'model DatasetOutline<'document>>,
    types: HashMap<&'model JsonPointer, &'model Typed>,
}

impl Fields<'_, '_> {
    /// The type of the field `reference` names, if it exists and E0.f typed it.
    fn reference_type(&self, reference: &FieldReference) -> Option<&Typed> {
        let field = self
            .by_name
            .get(&reference.dataset)?
            .field(&reference.field)?;
        self.types.get(&field.pointer).copied()
    }

    /// The type of the field `name` of the dataset at `dataset`, if it exists and E0.f typed it.
    fn field_type(&self, dataset: &JsonPointer, name: &str) -> Option<&Typed> {
        let field = self.by_pointer.get(dataset)?.field(&Name::declared(name))?;
        self.types.get(&field.pointer).copied()
    }
}

/// Checks every semi-additive declaration of `extensions`, in the semantic model that
/// `structure` selected, with the field types of `typing`.
///
/// # Errors
///
/// Returns at most one `E0C-12` and one `E0C-17` diagnostic per declaration, at the entry's
/// `data` member (E0C-2.6), in document order then by code. Both codes reject the model (E0-4).
pub fn type_additivity(
    structure: &Structure<'_>,
    typing: &Typing,
    extensions: &Extensions,
) -> Result<(), Vec<ExtensionDiagnostic>> {
    let model = outline(
        structure.semantic_model.node,
        &structure.semantic_model.pointer,
    );
    let fields = Fields {
        by_name: model
            .datasets
            .iter()
            .filter_map(|dataset| Some((&dataset.name.as_ref()?.name, dataset)))
            .collect(),
        by_pointer: model
            .datasets
            .iter()
            .map(|dataset| (&dataset.pointer, dataset))
            .collect(),
        types: typing
            .elements
            .iter()
            .filter(|element| element.kind == ElementKind::Field)
            .map(|element| (&element.pointer, &element.typed))
            .collect(),
    };
    let mut errors = Vec::new();
    for declaration in &extensions.additivity {
        let Some(keys) = declaration.additivity.semi_additive() else {
            continue;
        };
        let checks = [
            (
                ExtensionCode::AdditivityTimeKind,
                "an additivity option does not match the kind of `over`",
                time_kind_problems(&fields, keys),
            ),
            (
                ExtensionCode::AdditivityPopulation,
                "the population does not match the additivity declaration",
                population_problems(&fields, &extensions.populations, keys),
            ),
        ];
        for (code, summary, problems) in checks {
            if !problems.is_empty() {
                errors.push(diagnostic(declaration, code, summary, &problems));
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn diagnostic(
    declaration: &AdditivityDeclaration,
    code: ExtensionCode,
    summary: &str,
    problems: &[String],
) -> ExtensionDiagnostic {
    ExtensionDiagnostic {
        code,
        pointer: declaration.data.clone(),
        location: declaration.location,
        message: format!("{summary}: {}", problems.join("; ")),
    }
}

/// `E0C-12` (ADD-5.1.5): with a time `over`, the time options are required; with any other
/// `over`, they and every option that needs time order are not allowed.
fn time_kind_problems(fields: &Fields<'_, '_>, keys: &SemiAdditive) -> Vec<String> {
    let Some(typed) = fields.reference_type(&keys.over) else {
        return Vec::new();
    };
    let time = matches!(
        typed.logical(),
        Some(LogicalType::Date | LogicalType::Timestamp { .. })
    );
    let time_options = [
        ("snapshot_grain", keys.snapshot_grain.is_some()),
        ("population", keys.population.is_some()),
        ("missing", keys.missing.is_some()),
        ("unexpected", keys.unexpected.is_some()),
    ];
    if time {
        return time_options
            .iter()
            .filter(|(_, present)| !present)
            .map(|(key, _)| format!("`{key}` is required with the time field `over`"))
            .collect();
    }
    let kind = describe(&typed.value_type);
    let ordered = [
        (
            "`aggregation` `first` or `last` with `selection`",
            keys.aggregation.needs_time_order(),
        ),
        ("`nulls: as_missing`", keys.nulls == NullHandling::AsMissing),
    ];
    time_options
        .iter()
        .map(|(key, present)| (format!("`{key}`"), *present))
        .chain(ordered.map(|(key, present)| (key.to_owned(), present)))
        .filter(|(_, present)| *present)
        .map(|(key, _)| format!("{key} needs a time field `over`, and `over` is {kind}"))
        .collect()
}

/// `E0C-17` (ADD-5.1.7): `population` names a population of the model whose `entity` matches
/// the declaration's in number, order and logical kind, and a `periodic` population has the
/// `snapshot_grain` of the declaration.
fn population_problems(
    fields: &Fields<'_, '_>,
    populations: &[Population],
    keys: &SemiAdditive,
) -> Vec<String> {
    let Some(name) = &keys.population else {
        return Vec::new();
    };
    let wanted = Name::declared(name);
    let Some(population) = populations
        .iter()
        .find(|population| Name::declared(&population.name) == wanted)
    else {
        return vec![format!("`{name}` names no population of the model")];
    };
    let mut problems = entity_problems(fields, population, keys);
    if let (PopulationMode::Periodic { grain, .. }, Some(snapshot_grain)) =
        (&population.mode, keys.snapshot_grain)
        && *grain != snapshot_grain
    {
        problems.push(format!(
            "the periodic population `{}` has the grain {grain:?}, not the snapshot_grain {snapshot_grain:?}",
            population.name
        ));
    }
    problems
}

fn entity_problems(
    fields: &Fields<'_, '_>,
    population: &Population,
    keys: &SemiAdditive,
) -> Vec<String> {
    if population.entity.len() != keys.entity.len() {
        return vec![format!(
            "the population `{}` has {} entity fields, and the declaration {}",
            population.name,
            population.entity.len(),
            keys.entity.len()
        )];
    }
    keys.entity
        .iter()
        .zip(&population.entity)
        .filter_map(|(declared, expected)| {
            let declared_type = fields.reference_type(declared)?;
            let expected_type = fields.field_type(&population.dataset, expected)?;
            (!same_kind(&declared_type.value_type, &expected_type.value_type)).then(|| {
                format!(
                    "`{}` ({}) does not have the kind of `{expected}` ({}) of the population `{}`",
                    declared.written,
                    describe(&declared_type.value_type),
                    describe(&expected_type.value_type),
                    population.name
                )
            })
        })
        .collect()
}

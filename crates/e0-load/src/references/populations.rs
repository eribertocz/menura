//! `E0C-22`: every field a population names is a field of the dataset that declares it
//! (POP-4.3), in `entity`, in its time fields and inside `required_when`.
//!
//! A field name is read as a declared name (E0D-1.4) and compared under E0D-1.3. In
//! `required_when` a bare name is a field of the dataset (POP-4.4); a text that does not parse
//! and a qualified reference are `E0C-23`, reported by E0.f, not here.

use std::collections::HashMap;

use super::walk::references;
use crate::expressions::{ExpressionPlace, parse_element_expression};
use crate::extensions::{ExtensionCode, ExtensionDiagnostic, Extensions, Population, per_entry};
use crate::names::Name;
use crate::outline::{DatasetOutline, outline};
use crate::pointer::JsonPointer;
use crate::structure::Structure;

/// Resolves the fields of every population of `extensions`, in the semantic model that
/// `structure` selected.
///
/// # Errors
///
/// Returns one `E0C-22` diagnostic per entry whose populations name a missing field, at the
/// entry's `data` member (E0C-2.6), in document order. `E0C-22` rejects the model (E0-4).
pub fn resolve_population_references(
    structure: &Structure<'_>,
    extensions: &Extensions,
) -> Result<(), Vec<ExtensionDiagnostic>> {
    let model = outline(
        structure.semantic_model.node,
        &structure.semantic_model.pointer,
    );
    let datasets: HashMap<&JsonPointer, &DatasetOutline<'_>> = model
        .datasets
        .iter()
        .map(|dataset| (&dataset.pointer, dataset))
        .collect();
    let errors = per_entry(
        &extensions.populations,
        ExtensionCode::MissingPopulationField,
        "a population names a field its dataset does not have",
        |population| match datasets.get(&population.dataset) {
            Some(dataset) => missing_fields(dataset, population),
            None => Vec::new(),
        },
    );
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// The fields `population` names that `dataset` does not have, described for a message.
fn missing_fields(dataset: &DatasetOutline<'_>, population: &Population) -> Vec<String> {
    let mut missing: Vec<String> = Vec::new();
    for field in population.named_fields() {
        let described = format!("`{field}` in the population `{}`", population.name);
        if dataset.field(&Name::declared(field)).is_none() && !missing.contains(&described) {
            missing.push(described);
        }
    }
    for field in missing_condition_fields(dataset, population) {
        if !missing.contains(&field) {
            missing.push(field);
        }
    }
    missing
}

/// The bare names of `required_when` that are not fields of `dataset`, each once.
fn missing_condition_fields(dataset: &DatasetOutline<'_>, population: &Population) -> Vec<String> {
    let Some(text) = &population.required_when else {
        return Vec::new();
    };
    let Ok(tree) = parse_element_expression(text, ExpressionPlace::Extension) else {
        return Vec::new();
    };
    let mut missing: Vec<String> = Vec::new();
    for reference in references(&tree) {
        let absent =
            reference.qualifier.is_none() && dataset.field(&Name::of(&reference.name)).is_none();
        let described = format!(
            "`{}` in the required_when of the population `{}`",
            reference.name.text(),
            population.name
        );
        if absent && !missing.contains(&described) {
            missing.push(described);
        }
    }
    missing
}

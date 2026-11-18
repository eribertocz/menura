//! `E0C-16`: the fields of `over` and `entity` of a semi-additive metric exist and are reachable
//! from the metric's single home dataset (ADD-5.1.6). The home dataset of a dataset metric of
//! the development format is its own dataset (E0G-1.4).

use std::collections::{BTreeSet, HashSet};

use super::home_datasets::home_datasets;
use super::output::Relationships;
use super::reach::Model;
use crate::extensions::additivity::{AdditivityDeclaration, FieldReference};
use crate::extensions::{ExtensionCode, ExtensionDiagnostic, Extensions};
use crate::names::Name;
use crate::outline::outline;
use crate::pointer::JsonPointer;
use crate::references::References;
use crate::structure::Structure;

/// Checks the fields of every semi-additive declaration of `extensions`, in the semantic model
/// that `structure` selected, with the elements `references` made unusable and the graph of
/// `relationships`.
///
/// A metric made unusable before E0.e, or whose home datasets are none or several, is not
/// checked (ADD-5.1.6).
///
/// # Errors
///
/// Returns one `E0C-16` diagnostic per declaration with a field that does not exist or is not
/// reachable, at the entry's `data` member (E0C-2.6), in document order. `E0C-16` rejects the
/// model (E0-4).
pub(crate) fn check_additivity_reachability(
    structure: &Structure<'_>,
    extensions: &Extensions,
    references: &References,
    relationships: &Relationships,
) -> Result<(), Vec<ExtensionDiagnostic>> {
    let outline = outline(
        structure.semantic_model.node,
        &structure.semantic_model.pointer,
    );
    let model = Model::new(structure, &outline, references, relationships);
    let errors: Vec<ExtensionDiagnostic> = extensions
        .additivity
        .iter()
        .filter_map(|declaration| unreachable_fields(&model, declaration))
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// The single home dataset of the metric at `metric`: the dataset of a dataset metric
/// (E0G-1.4), or the one dataset the aggregate arguments of a model metric read (E0G-1.1).
fn home_dataset(model: &Model<'_, '_>, metric: &JsonPointer) -> Option<Name> {
    if let Some(name) = model.dataset_metrics.get(metric) {
        return Some((*name).clone());
    }
    single(home_datasets(model.metrics.get(metric)?))
}

/// The `E0C-16` diagnostic of `declaration`, if one of its fields does not exist or is not
/// reachable from the single home dataset of its metric.
fn unreachable_fields(
    model: &Model<'_, '_>,
    declaration: &AdditivityDeclaration,
) -> Option<ExtensionDiagnostic> {
    let keys = declaration.additivity.semi_additive()?;
    if model.unusable.contains(&declaration.metric) {
        return None;
    }
    let home = home_dataset(model, &declaration.metric)?;
    let home_dataset = model.datasets.get(&home)?;
    if model.unusable.contains(&home_dataset.pointer) {
        return None;
    }
    let reached = model.reached_from(&home);
    let problems: Vec<String> = std::iter::once(&keys.over)
        .chain(&keys.entity)
        .filter_map(|field| field_problem(model, &home, &reached, field))
        .collect();
    (!problems.is_empty()).then(|| ExtensionDiagnostic {
        code: ExtensionCode::UnreachableAdditivityField,
        pointer: declaration.data.clone(),
        location: declaration.location,
        message: format!(
            "a field of the additivity declaration is not reachable from the home dataset `{}`: {}",
            home.as_str(),
            problems.join("; ")
        ),
    })
}

fn single(names: BTreeSet<Name>) -> Option<Name> {
    let mut names = names.into_iter();
    match (names.next(), names.next()) {
        (Some(name), None) => Some(name),
        _ => None,
    }
}

/// Why `field` is not a field of `home` or of a dataset reached from it.
fn field_problem(
    model: &Model<'_, '_>,
    home: &Name,
    reached: &HashSet<Name>,
    field: &FieldReference,
) -> Option<String> {
    let Some(dataset) = model.datasets.get(&field.dataset) else {
        return Some(format!("`{}` names no dataset", field.written));
    };
    if dataset.field(&field.field).is_none() {
        return Some(format!("`{}` names no field of its dataset", field.written));
    }
    if field.dataset != *home && !reached.contains(&field.dataset) {
        return Some(format!(
            "no path leads from the home dataset to the dataset of `{}`",
            field.written
        ));
    }
    None
}

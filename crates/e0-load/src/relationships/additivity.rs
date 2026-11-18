//! `E0C-16`: the fields of `over` and `entity` of a semi-additive metric exist and are reachable
//! from the metric's single home dataset (ADD-5.1.6).

use std::collections::{BTreeSet, HashMap, HashSet};

use semantic_compiler_model::expression::Expression;

use super::home_datasets::home_datasets;
use super::output::{DatasetIndex, Relationships};
use crate::expressions::{ExpressionPlace, parse_element_expression};
use crate::extensions::additivity::{AdditivityDeclaration, FieldReference};
use crate::extensions::{ExtensionCode, ExtensionDiagnostic, Extensions};
use crate::names::Name;
use crate::outline::{DatasetOutline, ModelOutline, outline};
use crate::pointer::JsonPointer;
use crate::references::References;
use crate::structure::{CompiledVariant, ElementExpression, ElementKind, Structure};

/// What the reachability check reads.
struct Model<'model, 'document> {
    /// The datasets of the model, by name.
    datasets: HashMap<&'model Name, &'model DatasetOutline<'document>>,
    /// The metrics usable after E0.d, by JSON Pointer, with their compiled tree.
    metrics: HashMap<&'model JsonPointer, Expression>,
    /// The elements E0.d made unusable.
    unusable: HashSet<&'model JsonPointer>,
    relationships: &'model Relationships,
}

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
    let model = Model {
        datasets: named_datasets(&outline),
        metrics: structure
            .expressions
            .iter()
            .filter_map(metric_tree)
            .collect(),
        unusable: references
            .unusable
            .iter()
            .map(|element| &element.pointer)
            .collect(),
        relationships,
    };
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

fn named_datasets<'model, 'document>(
    outline: &'model ModelOutline<'document>,
) -> HashMap<&'model Name, &'model DatasetOutline<'document>> {
    outline
        .datasets
        .iter()
        .filter_map(|dataset| Some((&dataset.name.as_ref()?.name, dataset)))
        .collect()
}

/// The tree of a usable model metric that parses. A metric made unusable in E0.b is left out
/// (E0-4.1), and so is one that does not parse: it has no home dataset.
fn metric_tree<'structure>(
    expression: &'structure ElementExpression<'_>,
) -> Option<(&'structure JsonPointer, Expression)> {
    if expression.kind != ElementKind::Metric || !expression.is_usable() {
        return None;
    }
    let (CompiledVariant::AnsiSql(text) | CompiledVariant::OssieSql2026(text)) = expression.variant
    else {
        return None;
    };
    let tree = parse_element_expression(text, ExpressionPlace::Metric).ok()?;
    Some((&expression.pointer, tree))
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
    let tree = model.metrics.get(&declaration.metric)?;
    let home = single(home_datasets(tree))?;
    let home_dataset = model.datasets.get(&home)?;
    if model.unusable.contains(&home_dataset.pointer) {
        return None;
    }
    let reached = reached_from(model, &home);
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

/// The names of the datasets a path leads to from `home` (E0E-2.3, E0E-2.4).
fn reached_from(model: &Model<'_, '_>, home: &Name) -> HashSet<Name> {
    let graph = &model.relationships.graph;
    let Some(source) = graph
        .datasets
        .iter()
        .position(|dataset| dataset.name == *home)
    else {
        return HashSet::new();
    };
    model
        .relationships
        .reachable
        .get(source)
        .into_iter()
        .flatten()
        .filter_map(|reached| dataset_name(model, reached.dataset))
        .collect()
}

fn dataset_name(model: &Model<'_, '_>, index: DatasetIndex) -> Option<Name> {
    model
        .relationships
        .graph
        .datasets
        .get(index.0)
        .map(|dataset| dataset.name.clone())
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

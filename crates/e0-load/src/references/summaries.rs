//! `E0C-50`, `E0C-51` and `E0C-54`: the names of every summary, in E0.d (SUM-6.1.2).
//!
//! `of` names a dataset that is not a summary; `field` and `coverage_field` name fields of the
//! summary; `base` names a field of the `of` dataset; every field reference of a
//! `base_aggregate` names a field of a dataset that is not a summary (SUM-6.1.7). All are
//! declared names (E0D-1.4) compared under E0D-1.3. Every summary field is named by exactly one
//! dimension or measure, and `coverage_field` is the field of a dimension with `grain`. A
//! `base_aggregate` that does not parse, and a reference form other than `dataset.field`, are
//! left to E0.f (`E0C-52`, SUM-6.1.3).

use std::collections::HashMap;

use crate::expressions::{ExpressionPlace, parse_element_expression};
use crate::extensions::summaries::SummaryDeclaration;
use crate::extensions::{ExtensionCode, ExtensionDiagnostic, Extensions, Problems, report_at};
use crate::names::Name;
use crate::outline::{DatasetOutline, outline};
use crate::pointer::JsonPointer;
use crate::references::walk::references;
use crate::structure::Structure;

/// The datasets a summary may name: those of the model that are not summaries, by name.
struct Datasets<'model, 'document> {
    by_name: HashMap<&'model Name, &'model DatasetOutline<'document>>,
}

impl<'model, 'document> Datasets<'model, 'document> {
    /// The first dataset of each name of `datasets`, unless it is a summary of `extensions`.
    fn new(datasets: &'model [DatasetOutline<'document>], extensions: &Extensions) -> Self {
        let mut by_name = HashMap::new();
        for dataset in datasets {
            if let Some(name) = &dataset.name {
                by_name.entry(&name.name).or_insert(dataset);
            }
        }
        by_name.retain(|_, dataset| !extensions.is_summary(&dataset.pointer));
        Self { by_name }
    }

    /// The dataset named `name` that is not a summary.
    fn get(&self, name: &Name) -> Option<&'model DatasetOutline<'document>> {
        self.by_name.get(name).copied()
    }
}

/// Resolves the names of every summary declaration of `extensions`, in the semantic model that
/// `structure` selected.
///
/// # Errors
///
/// Returns at most one `E0C-50`, one `E0C-51` and one `E0C-54` diagnostic per summary, at the
/// `data` member of its entry of ours (E0C-2.6), in document order then by code. Each rejects
/// the model (E0-4).
pub fn resolve_summary_references(
    structure: &Structure<'_>,
    extensions: &Extensions,
) -> Result<(), Vec<ExtensionDiagnostic>> {
    let model = outline(
        structure.semantic_model.node,
        &structure.semantic_model.pointer,
    );
    let summaries: HashMap<&JsonPointer, &DatasetOutline<'_>> = model
        .datasets
        .iter()
        .map(|dataset| (&dataset.pointer, dataset))
        .collect();
    let datasets = Datasets::new(&model.datasets, extensions);
    let mut errors = Vec::new();
    for declaration in &extensions.summaries {
        let Some(summary) = summaries.get(&declaration.dataset) else {
            continue;
        };
        let problems = summary_problems(&datasets, summary, declaration);
        errors.extend(report_at(&declaration.data, declaration.location, problems));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// The problems of the names of `declaration`, the summary `summary`.
fn summary_problems(
    datasets: &Datasets<'_, '_>,
    summary: &DatasetOutline<'_>,
    declaration: &SummaryDeclaration,
) -> Problems {
    let mut problems = Problems::new();
    let base = datasets.get(&Name::declared(&declaration.of));
    if base.is_none() {
        problems.push((
            ExtensionCode::SummaryBase,
            format!(
                "`of` `{}` names no dataset of the model that is not a summary",
                declaration.of
            ),
        ));
    }
    problems.extend(field_problems(summary, base, declaration));
    problems.extend(aggregate_problems(datasets, declaration));
    problems.extend(coverage_problem(declaration));
    problems
}

/// `E0C-51` for every dimension and measure `field` that is not a field of `summary`, every
/// `base` that is not a field of `base`, when the `of` dataset exists, and every field of
/// `summary` named by no dimension and no measure or by more than one.
fn field_problems(
    summary: &DatasetOutline<'_>,
    base: Option<&DatasetOutline<'_>>,
    declaration: &SummaryDeclaration,
) -> Problems {
    let mut problems = Problems::new();
    let written = declaration
        .dimensions
        .iter()
        .map(|dimension| &dimension.field)
        .chain(declaration.measures.iter().map(|measure| &measure.field));
    for field in written.clone() {
        if summary.field(&Name::declared(field)).is_none() {
            problems.push(field_problem(format!(
                "`{field}` is not a field of the summary"
            )));
        }
    }
    if let Some(base) = base {
        for dimension in &declaration.dimensions {
            if base.field(&Name::declared(&dimension.base)).is_none() {
                problems.push(field_problem(format!(
                    "the `base` `{}` is not a field of `{}`",
                    dimension.base, declaration.of
                )));
            }
        }
    }
    let declared: Vec<Name> = written.map(|field| Name::declared(field)).collect();
    for field in &summary.fields {
        let Some(name) = &field.name else {
            continue;
        };
        let count = declared.iter().filter(|named| **named == name.name).count();
        if count != 1 {
            problems.push(field_problem(format!(
                "the summary field `{}` is named by {count} dimensions and measures, not one",
                name.written.text
            )));
        }
    }
    problems
}

/// `E0C-51` for every field reference of a `base_aggregate` that does not name a field of a
/// dataset that is not a summary.
fn aggregate_problems(datasets: &Datasets<'_, '_>, declaration: &SummaryDeclaration) -> Problems {
    let mut problems = Problems::new();
    for measure in &declaration.measures {
        let Ok(tree) = parse_element_expression(&measure.base_aggregate, ExpressionPlace::Metric)
        else {
            continue;
        };
        for reference in references(&tree) {
            let Some(qualifier) = &reference.qualifier else {
                continue;
            };
            let exists = datasets
                .get(&Name::of(qualifier))
                .is_some_and(|dataset| dataset.field(&Name::of(&reference.name)).is_some());
            if !exists {
                problems.push(field_problem(format!(
                    "the `base_aggregate` of `{}` refers to `{}`.`{}`, not a field of a dataset of the model",
                    measure.field,
                    qualifier.text(),
                    reference.name.text()
                )));
            }
        }
    }
    problems
}

/// `E0C-54` when `coverage_field` is not the `field` of a dimension that has `grain`.
fn coverage_problem(declaration: &SummaryDeclaration) -> Option<(ExtensionCode, String)> {
    let coverage = Name::declared(&declaration.coverage_field);
    let found = declaration
        .dimensions
        .iter()
        .any(|dimension| dimension.grain.is_some() && Name::declared(&dimension.field) == coverage);
    (!found).then(|| {
        (
            ExtensionCode::CoverageField,
            format!(
                "`coverage_field` `{}` is not the field of a dimension with `grain`",
                declaration.coverage_field
            ),
        )
    })
}

fn field_problem(message: String) -> (ExtensionCode, String) {
    (ExtensionCode::SummaryField, message)
}

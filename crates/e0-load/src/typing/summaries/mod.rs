//! `E0C-52`, `E0C-53` and `E0C-57`: the `base_aggregate` of every measure, the grain of every
//! dimension and the kind of every summary field, in E0.f (SUM-6.1.3 to SUM-6.1.5).
//!
//! A `base_aggregate` is parsed as a model metric expression. It is exactly one `SUM`,
//! `COUNT(x)`, `MIN` or `MAX` call without `DISTINCT`, with an optional `FILTER (WHERE …)`,
//! whose argument and filter are row-level expressions that read only fields of the `of`
//! dataset as `of.field`; anything else, and a `base_aggregate` of that shape that does not
//! type, is `E0C-52`. Its type is checked only when every field it reads has a type. A
//! dimension has `grain` exactly when its `base` has kind `date` or `timestamp` (`E0C-53`),
//! and every summary field has the kind of its base field or of its `base_aggregate` result
//! type (`E0C-57`), compared under E0F-4.1; both only on typed fields. A measure with `E0C-52`
//! is not checked for its kind.

mod base_aggregate;

use std::collections::HashMap;

use menura_contracts::types::common::LogicalType;

use super::keys::same_kind;
use super::output::{ElementKind, Typing};
use super::value_type::{Typed, describe};
use crate::extensions::summaries::{SummaryDeclaration, SummaryDimension};
use crate::extensions::{ExtensionCode, ExtensionDiagnostic, Extensions, Problems, report_at};
use crate::names::Name;
use crate::outline::{DatasetOutline, outline};
use crate::pointer::JsonPointer;
use crate::structure::Structure;
use base_aggregate::{MeasureType, measure_type};

/// The datasets and field types the checks of summaries read.
pub(super) struct Fields<'model, 'document> {
    by_pointer: HashMap<&'model JsonPointer, &'model DatasetOutline<'document>>,
    by_name: HashMap<&'model Name, &'model DatasetOutline<'document>>,
    types: HashMap<&'model JsonPointer, &'model Typed>,
}

impl<'model, 'document> Fields<'model, 'document> {
    /// The dataset named `name`, the first of that name.
    pub(super) fn dataset(&self, name: &Name) -> Option<&'model DatasetOutline<'document>> {
        self.by_name.get(name).copied()
    }

    /// The type of the field `name` of `dataset`, if it exists and E0.f typed it.
    pub(super) fn type_of(&self, dataset: &DatasetOutline<'_>, name: &Name) -> Option<&Typed> {
        let field = dataset.field(name)?;
        self.types.get(&field.pointer).copied()
    }
}

/// Types the measures and dimensions of every summary declaration of `extensions`, in the
/// semantic model that `structure` selected, with the field types of `typing`.
///
/// # Errors
///
/// Returns at most one `E0C-52`, one `E0C-53` and one `E0C-57` diagnostic per summary, at the
/// `data` member of its entry of ours (E0C-2.6), in document order then by code. Each rejects
/// the model (E0-4).
pub fn type_summaries(
    structure: &Structure<'_>,
    typing: &Typing,
    extensions: &Extensions,
) -> Result<(), Vec<ExtensionDiagnostic>> {
    let model = outline(
        structure.semantic_model.node,
        &structure.semantic_model.pointer,
    );
    let mut by_name = HashMap::new();
    for dataset in &model.datasets {
        if let Some(name) = &dataset.name {
            by_name.entry(&name.name).or_insert(dataset);
        }
    }
    let fields = Fields {
        by_pointer: model
            .datasets
            .iter()
            .map(|dataset| (&dataset.pointer, dataset))
            .collect(),
        by_name,
        types: typing
            .elements
            .iter()
            .filter(|element| element.kind == ElementKind::Field)
            .map(|element| (&element.pointer, &element.typed))
            .collect(),
    };
    let mut errors = Vec::new();
    for declaration in &extensions.summaries {
        let summary = fields.by_pointer.get(&declaration.dataset).copied();
        let base = fields.dataset(&Name::declared(&declaration.of));
        let (Some(summary), Some(base)) = (summary, base) else {
            continue;
        };
        let problems = summary_problems(&fields, summary, base, declaration);
        errors.extend(report_at(&declaration.data, declaration.location, problems));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// The problems of the measures and dimensions of `declaration`, the summary `summary` of the
/// dataset `base`.
fn summary_problems(
    fields: &Fields<'_, '_>,
    summary: &DatasetOutline<'_>,
    base: &DatasetOutline<'_>,
    declaration: &SummaryDeclaration,
) -> Problems {
    let mut problems = Problems::new();
    for measure in &declaration.measures {
        let result = match measure_type(fields, base, &declaration.of, measure) {
            MeasureType::Typed(result) => result,
            MeasureType::Invalid(message) => {
                problems.push((
                    ExtensionCode::InvalidBaseAggregate,
                    format!("the `base_aggregate` of `{}`: {message}", measure.field),
                ));
                continue;
            }
            MeasureType::Unchecked => continue,
        };
        let field = fields.type_of(summary, &Name::declared(&measure.field));
        problems.extend(kind_problem(
            &measure.field,
            field,
            &result,
            "its `base_aggregate`",
        ));
    }
    for dimension in &declaration.dimensions {
        let Some(base_type) = fields.type_of(base, &Name::declared(&dimension.base)) else {
            continue;
        };
        problems.extend(grain_problem(dimension, base_type));
        let field = fields.type_of(summary, &Name::declared(&dimension.field));
        let what = format!("its base `{}`", dimension.base);
        problems.extend(kind_problem(&dimension.field, field, base_type, &what));
    }
    problems
}

/// `E0C-53` when `dimension` has `grain` and its base, of type `base_type`, is not a time
/// field, or the other way round (SUM-6.1.4).
fn grain_problem(
    dimension: &SummaryDimension,
    base_type: &Typed,
) -> Option<(ExtensionCode, String)> {
    let temporal = matches!(
        base_type.logical(),
        Some(LogicalType::Date | LogicalType::Timestamp { .. })
    );
    let message = match (temporal, dimension.grain.is_some()) {
        (true, false) => format!(
            "the dimension `{}` reads a time field and has no `grain`",
            dimension.field
        ),
        (false, true) => format!(
            "the dimension `{}` has `grain`, but its base is {}, not a time field",
            dimension.field,
            describe(&base_type.value_type)
        ),
        _ => return None,
    };
    Some((ExtensionCode::SummaryGrain, message))
}

/// `E0C-57` when the summary field `name`, of type `field`, does not have the kind of
/// `expected`, the type of `what` (SUM-6.1.5). Nothing when the field has no type.
fn kind_problem(
    name: &str,
    field: Option<&Typed>,
    expected: &Typed,
    what: &str,
) -> Option<(ExtensionCode, String)> {
    let field = field?;
    (!same_kind(&field.value_type, &expected.value_type)).then(|| {
        (
            ExtensionCode::SummaryKind,
            format!(
                "the summary field `{name}` is {}, but {what} is {}",
                describe(&field.value_type),
                describe(&expected.value_type)
            ),
        )
    })
}

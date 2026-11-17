//! Types every field and metric expression (E0F-1, E0F-3 to E0F-7): fields against the columns
//! of their catalog relation, metrics against the types of the fields they read.

use std::collections::HashMap;

use semantic_compiler_contracts::types::common::LogicalType;
use semantic_compiler_model::expression::Reference;

use super::diagnostic::{TypingCode, TypingDiagnostic};
use super::output::ElementKind;
use super::scope::{Relation, Scope};
use super::stop::{Outcome, Stop, problem};
use super::typer::{Aggregates, ExpressionTyper};
use super::value_type::Typed;
use crate::names::Name;
use crate::outline::ElementOutline;
use crate::pointer::JsonPointer;

/// One typing problem and the element it makes unusable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Finding {
    /// The diagnostic.
    pub(crate) diagnostic: TypingDiagnostic,
    /// The element made unusable.
    pub(crate) element: (ElementKind, JsonPointer),
}

/// The types found for the elements of one kind, and the problems found.
#[derive(Debug, Default)]
pub(crate) struct ElementTypes<'model> {
    /// The type of every element that could be typed, by element pointer.
    pub(crate) types: HashMap<&'model JsonPointer, Typed>,
    /// One finding per element with a typing problem.
    pub(crate) findings: Vec<Finding>,
}

/// Types every field whose dataset's `source` the catalog describes.
pub(crate) fn type_fields<'model>(scope: &Scope<'model, '_>) -> ElementTypes<'model> {
    let mut found = ElementTypes::default();
    for dataset in &scope.model.datasets {
        let Some(relation) = scope.relation(dataset) else {
            continue;
        };
        let resolve = |reference: &Reference| column_type(relation, reference);
        for field in &dataset.fields {
            let typer = ExpressionTyper::new(&resolve, Aggregates::NotAllowed);
            record(scope, &typer, field, ElementKind::Field, &mut found);
        }
    }
    found
}

/// Types every metric, reading the types of `fields`.
pub(crate) fn type_metrics<'model>(
    scope: &Scope<'model, '_>,
    fields: &HashMap<&JsonPointer, Typed>,
) -> ElementTypes<'model> {
    let mut found = ElementTypes::default();
    let resolve = |reference: &Reference| field_type(scope, fields, reference);
    for metric in &scope.model.metrics {
        let typer = ExpressionTyper::new(&resolve, Aggregates::Allowed);
        record(scope, &typer, metric, ElementKind::Metric, &mut found);
    }
    found
}

/// Types the expression of `element`, if it parses, and records its type or its problem.
fn record<'model>(
    scope: &Scope<'model, '_>,
    expression_typer: &ExpressionTyper<'_>,
    element: &'model ElementOutline<'_>,
    kind: ElementKind,
    found: &mut ElementTypes<'model>,
) {
    let Some(tree) = scope.tree(&element.pointer) else {
        return;
    };
    match expression_typer.type_of(tree) {
        Ok(typed) => {
            found.types.insert(&element.pointer, typed);
        }
        Err(Stop::Problem { code, message }) => found.findings.push(Finding {
            diagnostic: TypingDiagnostic {
                code,
                pointer: element.expression_pointer.clone(),
                location: element.expression_location,
                message,
            },
            element: (kind, element.pointer.clone()),
        }),
        Err(Stop::ReportedElsewhere | Stop::Undecided) => {}
    }
}

/// The type of a column read by a field expression: an unqualified reference to a column of
/// the dataset's catalog relation. Any other reference is reported by E0.d.
fn column_type(relation: &Relation<'_>, reference: &Reference) -> Outcome<Typed> {
    if reference.qualifier.is_some() {
        return Err(Stop::ReportedElsewhere);
    }
    let column = relation
        .find_identifier(&reference.name)
        .ok_or(Stop::ReportedElsewhere)?;
    if column.logical_type == LogicalType::Unsupported {
        return problem(
            TypingCode::UnsupportedColumn,
            format!(
                "the column `{}` has the type `{}`, which is unsupported (CAT-4.3)",
                column.name, column.physical_type
            ),
        );
    }
    Ok(Typed::value(column.logical_type.clone(), column.nullable))
}

/// The type of a field read by a metric expression as `dataset.field`. A missing field is
/// reported by E0.d, and a field without a type by its own check.
fn field_type(
    scope: &Scope<'_, '_>,
    fields: &HashMap<&JsonPointer, Typed>,
    reference: &Reference,
) -> Outcome<Typed> {
    let qualifier = reference
        .qualifier
        .as_ref()
        .ok_or(Stop::ReportedElsewhere)?;
    scope
        .dataset(&Name::of(qualifier))
        .and_then(|dataset| dataset.field(&Name::of(&reference.name)))
        .and_then(|field| fields.get(&field.pointer))
        .cloned()
        .ok_or(Stop::ReportedElsewhere)
}

//! `E0D-03` and `E0D-04`: the references of field and metric expressions exist (E0D-3), and
//! `E0F-02` for a reference form E0D-3 does not list (E0D-3.4).
//!
//! A field expression reads columns of the catalog relation of its dataset's `source` with
//! unqualified references, never other fields; a metric expression reads `dataset.field`. A
//! qualified reference in a field expression, or an unqualified one in a metric expression, is
//! outside the accepted subset.
//!
//! The dataset-scoped metrics of the development format have their own forms, checked in
//! `dataset_metric_references`.

use semantic_compiler_model::expression::Reference;

use super::diagnostic::ReferenceCode;
use super::finding::Finding;
use super::name::Name;
use super::outline::ElementOutline;
use super::output::ElementKind;
use super::scope::{Scope, field};
use super::walk::references;

/// Every `E0D-03` finding of the model: one per field that reads a missing column.
///
/// The fields of a dataset whose `source` the catalog does not describe are not checked: the
/// model is already rejected by `E0D-02`.
pub(crate) fn column_findings(scope: &Scope<'_, '_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    for dataset in &scope.model.datasets {
        let Some(columns) = scope.columns(dataset) else {
            continue;
        };
        for element in &dataset.fields {
            let missing = missing_names(scope, element, |reference| {
                reference.qualifier.is_none() && columns.find_identifier(&reference.name).is_none()
            });
            if let Some(missing) = missing {
                findings.push(finding(
                    ReferenceCode::UnknownColumn,
                    element,
                    format!(
                        "the field expression reads {missing}, not a column of its catalog relation"
                    ),
                    ElementKind::Field,
                ));
            }
        }
    }
    findings
}

/// Every `E0D-04` finding of the model metrics: one per metric that refers to a missing dataset
/// or field.
pub(crate) fn metric_findings(scope: &Scope<'_, '_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    for element in &scope.model.metrics {
        let missing = missing_names(scope, element, |reference| {
            reference.qualifier.as_ref().is_some_and(|qualifier| {
                !exists(scope, &Name::of(qualifier), &Name::of(&reference.name))
            })
        });
        if let Some(missing) = missing {
            findings.push(finding(
                ReferenceCode::MissingMetricReference,
                element,
                format!("the metric refers to {missing}, not a field of a dataset of the model"),
                ElementKind::Metric,
            ));
        }
    }
    findings
}

/// Every `E0F-02` finding of E0D-3.4: one per field with a qualified reference and one per
/// metric with an unqualified reference.
pub(crate) fn form_findings(scope: &Scope<'_, '_>) -> Vec<Finding> {
    let fields = scope
        .model
        .datasets
        .iter()
        .flat_map(|dataset| &dataset.fields);
    let field_findings = fields.filter_map(|element| {
        let unlisted = missing_names(scope, element, |reference| reference.qualifier.is_some())?;
        Some(finding(
            ReferenceCode::UnlistedReferenceForm,
            element,
            format!("the field expression refers to {unlisted}; a field expression reads source columns by their bare name"),
            ElementKind::Field,
        ))
    });
    let metric_findings = scope.model.metrics.iter().filter_map(|element| {
        let unlisted = missing_names(scope, element, |reference| reference.qualifier.is_none())?;
        Some(finding(
            ReferenceCode::UnlistedReferenceForm,
            element,
            format!("the metric refers to {unlisted}; a metric expression reads fields as `dataset.field`"),
            ElementKind::Metric,
        ))
    });
    field_findings.chain(metric_findings).collect()
}

/// Whether the model has a dataset `dataset` with a field `name`.
fn exists(scope: &Scope<'_, '_>, dataset: &Name, name: &Name) -> bool {
    scope
        .dataset(dataset)
        .is_some_and(|dataset| field(dataset, name).is_some())
}

/// The references of `element` that `is_missing` selects, written for a message, or `None`
/// when there are none.
pub(super) fn missing_names(
    scope: &Scope<'_, '_>,
    element: &ElementOutline<'_>,
    is_missing: impl Fn(&Reference) -> bool,
) -> Option<String> {
    let tree = scope.tree(&element.pointer)?;
    let mut missing: Vec<String> = Vec::new();
    for reference in references(tree)
        .into_iter()
        .filter(|reference| is_missing(reference))
    {
        let written = written(reference);
        if !missing.contains(&written) {
            missing.push(written);
        }
    }
    (!missing.is_empty()).then(|| missing.join(", "))
}

/// A reference as the message shows it, such as `` `orders`.`amount` ``.
fn written(reference: &Reference) -> String {
    let name = format!("`{}`", reference.name.text());
    match &reference.qualifier {
        Some(qualifier) => format!("`{}`.{name}", qualifier.text()),
        None => name,
    }
}

/// A finding at the expression of `element` that makes it unusable as `kind`.
pub(super) fn finding(
    code: ReferenceCode,
    element: &ElementOutline<'_>,
    message: String,
    kind: ElementKind,
) -> Finding {
    Finding::missing(
        code,
        (&element.expression_pointer, element.expression_location),
        message,
        (kind, &element.pointer),
    )
}

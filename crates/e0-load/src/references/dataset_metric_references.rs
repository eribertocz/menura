//! The references of the dataset-scoped metrics of the development format (section 4.5; E0D-3,
//! the row of development-line dataset metric expressions).
//!
//! A bare name is a field of the metric's dataset (`E0D-04` if missing). `source.column` is a
//! column of the catalog relation of the dataset's `source` (`E0D-03` if missing); the
//! qualifier `source` is compared as a regular identifier (E0D-1.1, E0D-1.3). Any other
//! qualified reference is outside the accepted subset (`E0F-02`, E0D-3.4).

use semantic_compiler_model::expression::Reference;

use super::diagnostic::ReferenceCode;
use super::expression_references::{finding, missing_names};
use super::finding::Finding;
use super::name::Name;
use super::output::ElementKind;
use super::scope::{Scope, field};

/// The qualifier of a source column reference.
const SOURCE_QUALIFIER: &str = "source";

/// What a reference of a dataset metric names.
enum Form {
    /// A bare name: a field of the metric's dataset.
    Field,
    /// `source.column`: a column of the catalog relation of the dataset's `source`.
    SourceColumn,
    /// Any other qualified reference.
    Unlisted,
}

/// Every finding of the dataset metrics of the model: per metric, one `E0D-03` for missing
/// source columns, one `E0D-04` for missing fields and one `E0F-02` for unlisted forms.
///
/// Source columns are not checked for a dataset whose `source` the catalog does not describe:
/// the model is already rejected by `E0D-02`.
pub(crate) fn dataset_metric_findings(scope: &Scope<'_, '_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    for dataset in &scope.model.datasets {
        for element in &dataset.metrics {
            let mut report = |code, missing: Option<String>, describe: fn(&str) -> String| {
                if let Some(missing) = missing {
                    findings.push(finding(
                        code,
                        element,
                        describe(&missing),
                        ElementKind::Metric,
                    ));
                }
            };
            let columns = scope.columns(dataset);
            let missing_columns = missing_names(scope, element, |reference| {
                matches!(form(reference), Form::SourceColumn)
                    && columns
                        .is_some_and(|columns| columns.find_identifier(&reference.name).is_none())
            });
            report(ReferenceCode::UnknownColumn, missing_columns, |missing| {
                format!("the dataset metric reads {missing}, not a column of its catalog relation")
            });
            let missing_fields = missing_names(scope, element, |reference| {
                matches!(form(reference), Form::Field)
                    && field(dataset, &Name::of(&reference.name)).is_none()
            });
            report(
                ReferenceCode::MissingMetricReference,
                missing_fields,
                |missing| {
                    format!("the dataset metric refers to {missing}, not a field of its dataset")
                },
            );
            let unlisted = missing_names(scope, element, |reference| {
                matches!(form(reference), Form::Unlisted)
            });
            report(ReferenceCode::UnlistedReferenceForm, unlisted, |unlisted| {
                format!(
                    "the dataset metric refers to {unlisted}; a dataset metric reads fields by their bare name and source columns as `source.column`"
                )
            });
        }
    }
    findings
}

/// The form of a reference in a dataset metric.
fn form(reference: &Reference) -> Form {
    match &reference.qualifier {
        None => Form::Field,
        Some(qualifier) if Name::of(qualifier) == Name::regular(SOURCE_QUALIFIER) => {
            Form::SourceColumn
        }
        Some(_) => Form::Unlisted,
    }
}

//! `E0D-05` to `E0D-07`: relationships name datasets of the model, and every key or
//! relationship column resolves (E0D-3, E0D-3.1).

use menura_model::expression::Expression;

use super::diagnostic::ReferenceCode;
use super::finding::Finding;
use super::output::ElementKind;
use super::scope::Scope;
use crate::columns::Columns;
use crate::outline::{DatasetOutline, DeclaredName, RelationshipOutline};
use crate::pointer::JsonPointer;

/// Every finding of the keys of every dataset, which make the dataset unusable.
pub(crate) fn dataset_key_findings(scope: &Scope<'_, '_>) -> Vec<Finding> {
    scope
        .model
        .datasets
        .iter()
        .flat_map(|dataset| {
            let element = (ElementKind::Dataset, &dataset.pointer);
            dataset
                .keys
                .iter()
                .filter_map(move |key| resolve(scope, dataset, key, element))
        })
        .collect()
}

/// Every finding of every relationship, which make the relationship unusable.
pub(crate) fn relationship_findings(scope: &Scope<'_, '_>) -> Vec<Finding> {
    scope
        .model
        .relationships
        .iter()
        .flat_map(|relationship| one_relationship(scope, relationship))
        .collect()
}

fn one_relationship(scope: &Scope<'_, '_>, relationship: &RelationshipOutline<'_>) -> Vec<Finding> {
    let element = (ElementKind::Relationship, &relationship.pointer);
    let mut findings = Vec::new();
    for (end, columns) in [
        (&relationship.from, &relationship.from_columns),
        (&relationship.to, &relationship.to_columns),
    ] {
        let Some(end) = end else {
            continue;
        };
        let Some(dataset) = scope.dataset(&end.name) else {
            findings.push(Finding::missing(
                ReferenceCode::MissingRelationshipDataset,
                end.written.place(),
                format!(
                    "the relationship refers to `{}`, not a dataset of the model",
                    end.written.text
                ),
                element,
            ));
            continue;
        };
        findings.extend(
            columns
                .iter()
                .filter_map(|column| resolve(scope, dataset, column, element)),
        );
    }
    findings
}

/// Resolves the key or relationship column `key` in `dataset` (E0D-3.1): to the field of that
/// name, otherwise to the source column of that name. Returns the finding when it does not
/// resolve.
///
/// When the catalog does not describe the dataset's `source`, only fields are looked up and a
/// name that is not a field is not reported: the model is already rejected by `E0D-02`. When a
/// field has the name but its expression cannot be used, whether it is exactly the column cannot
/// be decided, and nothing is reported: the field is already unusable (E0.b, E0.f).
fn resolve(
    scope: &Scope<'_, '_>,
    dataset: &DatasetOutline<'_>,
    key: &DeclaredName<'_>,
    element: (ElementKind, &JsonPointer),
) -> Option<Finding> {
    let columns = scope.columns(dataset);
    let column = columns.map(|columns| columns.find(&key.name, key.delimited));
    let dataset_name = dataset.name.as_ref().map_or("", |name| name.written.text);
    match (dataset.field(&key.name), column) {
        (Some(found), Some(Some(column))) => {
            let tree = scope.tree(&found.pointer)?;
            let reads_column = columns.is_some_and(|columns| reads_exactly(tree, columns, column));
            (!reads_column).then(|| {
                Finding::missing(
                    ReferenceCode::KeyFieldIsOtherColumn,
                    key.written.place(),
                    format!(
                        "`{}` is a field of `{dataset_name}` that is not exactly the source column `{column}`",
                        key.written.text
                    ),
                    element,
                )
            })
        }
        (None, Some(None)) => Some(Finding::missing(
            ReferenceCode::UnresolvedKeyName,
            key.written.place(),
            format!(
                "`{}` is neither a field of `{dataset_name}` nor a column of its source",
                key.written.text
            ),
            element,
        )),
        _ => None,
    }
}

/// Whether `tree` is exactly the source column spelled `column`: an unqualified reference that
/// matches that column (E0D-1.5).
fn reads_exactly(tree: &Expression, columns: &Columns<'_>, column: &str) -> bool {
    matches!(tree, Expression::Reference(reference)
        if reference.qualifier.is_none() && columns.find_identifier(&reference.name) == Some(column))
}

//! `E0D-01`: duplicate names (E0D-1.3, E0D-2, DEV-2), names longer than 128 characters
//! (E0D-1.2, DEV-9) and malformed delimited names (E0D-1.4).

use std::collections::HashSet;

use super::diagnostic::ReferenceCode;
use super::finding::Finding;
use crate::names::{MAXIMUM_LENGTH, identifier_text, is_malformed, is_too_long};
use crate::outline::{DeclaredName, ElementOutline, ModelOutline};

/// Every `E0D-01` finding of the document: the names of every semantic model, selected or not,
/// and the names inside `model`, the selected one.
///
/// Each kind of element has its own scope: semantic model names are unique within the
/// document; dataset, metric and relationship names within the model; field names within their
/// dataset. A metric and a dataset may share a name (E0D-2). A duplicate is reported at every
/// name after the first.
///
/// A malformed delimited name has no reading (E0D-1.4): it gets one finding of its own and is
/// left out of the length and duplicate checks. Key and relationship columns, and the names of
/// dataset metrics, are declared names too and are checked for it.
pub(crate) fn name_findings(
    semantic_models: &[DeclaredName<'_>],
    model: &ModelOutline<'_>,
) -> Vec<Finding> {
    let mut findings = unscoped_malformed_findings(model);
    scope(semantic_models.iter(), "semantic model", &mut findings);
    let dataset_names = model
        .datasets
        .iter()
        .filter_map(|dataset| dataset.name.as_ref());
    scope(dataset_names, "dataset", &mut findings);
    for dataset in &model.datasets {
        scope(
            names(&dataset.fields),
            "field of this dataset",
            &mut findings,
        );
    }
    let relationship_names = model
        .relationships
        .iter()
        .filter_map(|relationship| relationship.name.as_ref());
    scope(relationship_names, "relationship", &mut findings);
    scope(names(&model.metrics), "metric", &mut findings);
    findings
}

fn names<'outline, 'document>(
    elements: &'outline [ElementOutline<'document>],
) -> impl Iterator<Item = &'outline DeclaredName<'document>> {
    elements.iter().filter_map(|element| element.name.as_ref())
}

/// The findings of one scope of names, described as `kind`.
fn scope<'outline, 'document: 'outline>(
    names: impl Iterator<Item = &'outline DeclaredName<'document>>,
    kind: &str,
    findings: &mut Vec<Finding>,
) {
    let mut seen = HashSet::new();
    for name in names {
        if is_malformed(name.written.text) {
            findings.push(malformed(name));
            continue;
        }
        findings.extend(too_long(name));
        if !seen.insert(&name.name) {
            findings.push(Finding::rejecting(
                ReferenceCode::InvalidName,
                &name.written,
                format!(
                    "`{}` is the name of an earlier {kind}; names equal after normalization are the same name",
                    name.written.text
                ),
            ));
        }
    }
}

fn too_long(name: &DeclaredName<'_>) -> Option<Finding> {
    let text = identifier_text(name.written.text);
    is_too_long(&text).then(|| {
        Finding::rejecting(
            ReferenceCode::InvalidName,
            &name.written,
            format!(
                "the name has {} characters; an identifier has at most {MAXIMUM_LENGTH}",
                text.chars().count()
            ),
        )
    })
}

/// The findings of the malformed declared names that are not in a uniqueness scope: the key
/// columns and dataset metric names of every dataset, then the columns of every relationship.
fn unscoped_malformed_findings(model: &ModelOutline<'_>) -> Vec<Finding> {
    let dataset_names = model
        .datasets
        .iter()
        .flat_map(|dataset| dataset.keys.iter().chain(names(&dataset.metrics)));
    let relationship_columns = model.relationships.iter().flat_map(|relationship| {
        relationship
            .from_columns
            .iter()
            .chain(&relationship.to_columns)
    });
    dataset_names
        .chain(relationship_columns)
        .filter(|name| is_malformed(name.written.text))
        .map(malformed)
        .collect()
}

fn malformed(name: &DeclaredName<'_>) -> Finding {
    Finding::rejecting(
        ReferenceCode::InvalidName,
        &name.written,
        format!(
            "`{}` starts with a double quote but is not a delimited identifier, so the name is malformed; \
             a delimited name ends with a double quote, has at least one character between its quotes, \
             and doubles every double quote inside them",
            name.written.text
        ),
    )
}

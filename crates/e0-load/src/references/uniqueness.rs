//! `E0D-01`: duplicate names (E0D-1.3, E0D-2, DEV-2) and names longer than 128 characters
//! (E0D-1.2, DEV-9).

use std::collections::HashSet;

use super::diagnostic::ReferenceCode;
use super::finding::Finding;
use super::outline::{DeclaredName, ElementOutline, ModelOutline};
use crate::names::{MAXIMUM_LENGTH, identifier_text, is_too_long};

/// Every `E0D-01` finding of the document: the names of every semantic model, selected or not,
/// and the names inside `model`, the selected one.
///
/// Each kind of element has its own scope: semantic model names are unique within the
/// document; dataset, metric and relationship names within the model; field names within their
/// dataset. A metric and a dataset may share a name (E0D-2). A duplicate is reported at every
/// name after the first.
pub(crate) fn name_findings(
    semantic_models: &[DeclaredName<'_>],
    model: &ModelOutline<'_>,
) -> Vec<Finding> {
    let mut findings = Vec::new();
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

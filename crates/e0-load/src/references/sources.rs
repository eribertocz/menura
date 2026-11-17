//! `E0D-02`: every dataset `source` is described by the catalog schema, with identical text
//! (E0D-3, DEV-3).

use super::diagnostic::ReferenceCode;
use super::finding::Finding;
use super::scope::Scope;

/// Every `E0D-02` finding of the model.
pub(crate) fn source_findings(scope: &Scope<'_, '_>) -> Vec<Finding> {
    scope
        .model
        .datasets
        .iter()
        .filter_map(|dataset| dataset.source.as_ref())
        .filter(|source| !scope.describes(source.text))
        .map(|source| {
            Finding::rejecting(
                ReferenceCode::SourceNotInCatalog,
                source,
                format!(
                    "the catalog schema describes no relation with the source `{}`",
                    source.text
                ),
            )
        })
        .collect()
}

//! The access every dataset is read with, and the warnings of a missing `security_context` or
//! `access` (SCY-3.0, SCY-5.1.7, DEV-4).

use semantic_compiler_security::DatasetAccess;

use super::model::Security;
use crate::extensions::diagnostic::{ExtensionCode, ExtensionDiagnostic};
use crate::extensions::summaries::SummaryDatasets;
use crate::outline::outline;
use crate::pointer::{JsonPointer, resolve};
use crate::read::Node;
use crate::structure::SelectedModel;

/// The access of one dataset of the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetAccessOf {
    /// The JSON Pointer of the dataset.
    pub dataset: JsonPointer,
    /// Its access; `None` when it is inaccessible: it has no `access`, and it is a summary or
    /// the load option `grant_authenticated_to_all` is not set (SCY-3.0, SUM-6.1.6).
    pub access: Option<DatasetAccess>,
}

/// The access of every dataset of a model, with the load warnings about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAccess {
    /// One item per dataset, in document order.
    pub datasets: Vec<DatasetAccessOf>,
    /// `W0C-40` at the semantic model, then `W0C-41` at each dataset without `access` that is
    /// not a summary, in document order.
    pub warnings: Vec<ExtensionDiagnostic>,
}

/// The access of every dataset of the semantic model `model` of `document`, with the
/// `security` declarations E0.c read and the `summaries` of the model. With
/// `grant_authenticated_to_all` (DEV-4), a dataset without `access` is read as
/// `mode: authenticated` without `fields` and gets no `W0C-41` (SCY-5.1.7). A summary without
/// `access` is inaccessible and never gets `W0C-41`, whatever `grant_authenticated_to_all`
/// says (SUM-6.1.6): its missing `access` is `E0C-56`.
pub(crate) fn resolve_access(
    document: &Node,
    model: &SelectedModel<'_>,
    security: &Security,
    summaries: &SummaryDatasets,
    grant_authenticated_to_all: bool,
) -> ResolvedAccess {
    let mut warnings = Vec::new();
    if security.context.is_none() {
        warnings.push(warning(
            document,
            model,
            &security.model,
            ExtensionCode::NoSecurityContext,
            "the model has no `security_context`; it declares no roles or attributes".to_owned(),
        ));
    }
    let mut datasets = Vec::new();
    for dataset in outline(model.node, &model.pointer).datasets {
        let declared = security
            .access(&dataset.pointer)
            .map(|declared| declared.access.clone());
        let access = match declared {
            Some(access) => Some(access),
            None if summaries.contains(&dataset.pointer) => None,
            None if grant_authenticated_to_all => Some(DatasetAccess::granted_to_authenticated()),
            None => {
                let name = dataset.name.as_ref().map_or("", |name| name.written.text);
                warnings.push(warning(
                    document,
                    model,
                    &dataset.pointer,
                    ExtensionCode::NoAccess,
                    format!("the dataset `{name}` has no `access`; it is inaccessible"),
                ));
                None
            }
        };
        datasets.push(DatasetAccessOf {
            dataset: dataset.pointer,
            access,
        });
    }
    ResolvedAccess { datasets, warnings }
}

/// A warning with `code` at the node `pointer` of `document`, inside `model`.
fn warning(
    document: &Node,
    model: &SelectedModel<'_>,
    pointer: &JsonPointer,
    code: ExtensionCode,
    message: String,
) -> ExtensionDiagnostic {
    ExtensionDiagnostic {
        code,
        pointer: pointer.clone(),
        location: resolve(document, pointer).map_or(model.node.location, |node| node.location),
        message,
    }
}

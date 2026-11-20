//! Substage E0.d of the stage function: the references of the model's own elements, and,
//! only when E0.c succeeded, the references inside extensions (E0-3.3).

use menura_contracts::types::catalog::CatalogSchemaDocument;

use super::findings::{self, Substage};
use super::report::Report;
use crate::extensions::summaries::SummaryDatasets;
use crate::extensions::{ExtensionDiagnostic, Extensions};
use crate::read::Node;
use crate::references::context::{ContextMetrics, resolve_context_references};
use crate::references::derived::{DerivedMetrics, resolve_derived_references};
use crate::references::populations::resolve_population_references;
use crate::references::security::resolve_security_references;
use crate::references::summaries::resolve_summary_references;
use crate::references::{References, resolve_references};
use crate::structure::Structure;

/// What E0.d resolved, when it succeeded.
pub(super) struct Resolved {
    /// The references of the model's own elements.
    pub(super) references: References,
    /// The derived metrics and their inputs; empty when the extension checks did not run.
    pub(super) derived: DerivedMetrics,
    /// The context metrics and their bases; empty when the extension checks did not run.
    pub(super) context: ContextMetrics,
}

/// Runs E0.d and adds its diagnostics to `report`; `None` when E0.d fails, so the substages
/// that depend on it do not run (E0-3). `extensions` is `None` when E0.c failed: the extension
/// checks of E0.d are then skipped (E0-3.3).
pub(super) fn resolve(
    document: &Node,
    structure: &Structure<'_>,
    catalog: &CatalogSchemaDocument,
    summaries: &SummaryDatasets,
    extensions: Option<&Extensions>,
    report: &mut Report,
) -> Option<Resolved> {
    let mut substage = Substage::default();
    let references = match resolve_references(document, structure, catalog, summaries) {
        Ok(references) => {
            substage.extend(references.warnings.iter().map(findings::reference));
            Some(references)
        }
        Err(rejection) => {
            substage.extend(rejection.errors.iter().map(findings::reference));
            substage.extend(rejection.warnings.iter().map(findings::reference));
            None
        }
    };
    let resolved_extensions = extensions
        .map(|extensions| extension_references(document, structure, extensions, &mut substage));
    if report.add_substage(document, substage) {
        return None;
    }
    let (derived, context) = match resolved_extensions {
        Some((Some(derived), Some(context))) => (derived, context),
        _ => (DerivedMetrics::default(), ContextMetrics::default()),
    };
    Some(Resolved {
        references: references?,
        derived,
        context,
    })
}

/// Runs the extension checks of E0.d, adding their diagnostics to `substage`, and returns the
/// derived and context metrics they resolved.
fn extension_references(
    document: &Node,
    structure: &Structure<'_>,
    extensions: &Extensions,
    substage: &mut Substage,
) -> (Option<DerivedMetrics>, Option<ContextMetrics>) {
    add_errors(
        substage,
        resolve_population_references(structure, extensions),
    );
    let derived = match resolve_derived_references(document, structure, extensions) {
        Ok(derived) => {
            substage.extend(derived.warnings.iter().map(findings::extension));
            Some(derived)
        }
        Err(rejection) => {
            substage.extend(rejection.errors.iter().map(findings::extension));
            substage.extend(rejection.warnings.iter().map(findings::extension));
            None
        }
    };
    let context = match resolve_context_references(document, structure, extensions) {
        Ok(context) => Some(context),
        Err(errors) => {
            substage.extend(errors.iter().map(findings::extension));
            None
        }
    };
    add_errors(substage, resolve_security_references(structure, extensions));
    add_errors(substage, resolve_summary_references(structure, extensions));
    (derived, context)
}

/// Adds the diagnostics of an extension check that returns nothing else.
pub(super) fn add_errors(substage: &mut Substage, result: Result<(), Vec<ExtensionDiagnostic>>) {
    if let Err(errors) = result {
        substage.extend(errors.iter().map(findings::extension));
    }
}

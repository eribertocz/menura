//! The E0 stage function (STG-1, E0-2, E0-3): admits the catalog schema, runs the substages
//! E0.a to E0.h in order, each only when the substages it depends on succeeded, and reports
//! every diagnostic in the order of E0-4.3 (STG-3.4, STG-3.6).
//!
//! The catalog schema is admitted first, and its error comes first (E0-3.2). E0.a to E0.c run
//! without it. When E0.c fails, the checks of the model's own elements in E0.d to E0.g still
//! run and the extension checks of those substages do not (E0-3.3). E0.h runs only when no
//! substage failed.

mod analysis;
mod catalog;
mod early;
mod findings;
mod parsing;
mod report;
mod resolution;

use std::fmt;

use semantic_compiler_contracts::types::audit_record::LoadOptions;
use semantic_compiler_contracts::types::catalog::CatalogSchemaDocument;
use semantic_compiler_contracts::types::compiled_plan::Diagnostic;
use semantic_compiler_model::internal_model::InternalModel;

use crate::extensions::summaries::datasets::summary_datasets;
use crate::internal_model::{ModelInput, build_internal_model};
use crate::read::{Node, read_document};
use analysis::{Earlier, classification, relationships, types};
use early::{AcceptedExtensions, summaries_only};
use report::Report;
use resolution::resolve;

/// The model does not load: every error E0 found, and the warnings found with them (E0-2,
/// PLN-5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadRejection {
    /// The errors, never empty: the catalog schema error first (E0-3.2), then the errors of
    /// each substage in the order of E0-4.3.
    pub errors: Vec<Diagnostic>,
    /// The load warnings, in the order of E0-4.3.
    pub warnings: Vec<Diagnostic>,
}

impl fmt::Display for LoadRejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "the model does not load")?;
        for error in &self.errors {
            write!(formatter, "; {}: {}", error.code, error.message)?;
        }
        Ok(())
    }
}

impl std::error::Error for LoadRejection {}

/// Stage E0: loads and validates the model document `model` against the catalog schema
/// document `catalog_schema`, with the load options `options` (E0-2). Both documents are the
/// bytes as received; E0 needs no security context and no catalog state (E0-2.1).
///
/// Returns the immutable internal model, whose warnings are every load warning in the order of
/// E0-4.3, with the admitted catalog schema.
///
/// # Errors
///
/// A [`LoadRejection`] with every error and warning found, when the catalog schema is not
/// admitted (`ECAT-*`, CAT-9.1) or a substage reports a diagnostic whose disposition is
/// *reject* (E0-4.1).
///
/// # Example
///
/// ```
/// use semantic_compiler_contracts::types::audit_record::{LoadOptions, OssieFormat};
/// use semantic_compiler_e0_load::load_model;
///
/// let options = LoadOptions {
///     grant_authenticated_to_all: false,
///     ossie_format: OssieFormat::Release,
///     namespace: "semantic_compiler".to_owned(),
///     model_name: None,
/// };
/// let rejection = load_model(b"version: \"0.1.1\"", b"{}", &options).unwrap_err();
/// assert_eq!(rejection.errors[0].code, "ECAT-02");
/// ```
pub fn load_model(
    model: &[u8],
    catalog_schema: &[u8],
    options: &LoadOptions,
) -> Result<(InternalModel, CatalogSchemaDocument), LoadRejection> {
    let mut report = Report::default();
    let catalog = catalog::admit(catalog_schema, &mut report);
    let document = match read_document(model) {
        Ok(document) => document,
        Err(error) => {
            report.add_read_error(&error);
            return Err(report.into_rejection());
        }
    };
    let built = substages(&document, catalog.as_ref(), options, &mut report);
    match (built, catalog) {
        (Some(model), Some(catalog)) if !report.has_errors() => Ok((report.seal(&model), catalog)),
        _ => Err(report.into_rejection()),
    }
}

/// Runs E0.b to E0.h on the read `document`, adding every diagnostic to `report`; the
/// internal model when every substage ran and none failed.
fn substages(
    document: &Node,
    catalog: Option<&CatalogSchemaDocument>,
    options: &LoadOptions,
    report: &mut Report,
) -> Option<InternalModel> {
    let structure = early::structure(document, options, report)?;
    let summaries = summary_datasets(&structure.semantic_model, &options.namespace);
    let accepted = early::extensions(document, &structure, &summaries, options, report);
    let catalog = catalog?;
    let extensions = accepted.as_ref().map(|accepted| &accepted.extensions);
    let resolved = resolve(
        document, &structure, catalog, &summaries, extensions, report,
    )?;
    let own_only;
    let extensions = if let Some(extensions) = extensions {
        extensions
    } else {
        own_only = summaries_only(document, &summaries);
        &own_only
    };
    let earlier = Earlier {
        document,
        structure: &structure,
        catalog,
        summaries: &summaries,
        extensions,
        extension_checks: accepted.is_some(),
        resolved: &resolved,
    };
    let analyzed = relationships(&earlier, report);
    let typed = types(&earlier, analyzed.failed, report)?;
    let classified = classification(&earlier, &typed, report);
    let accepted = accepted.as_ref().filter(|_| !analyzed.failed)?;
    Some(internal_model(
        &earlier,
        accepted,
        &analyzed.relationships,
        &typed,
        &classified,
    ))
}

/// Runs E0.h.
fn internal_model(
    earlier: &Earlier<'_, '_>,
    accepted: &AcceptedExtensions,
    relationships: &crate::relationships::Relationships,
    typed: &analysis::Typed,
    classification: &crate::classification::Classification,
) -> InternalModel {
    let input = ModelInput {
        structure: earlier.structure,
        extensions: &accepted.extensions,
        access: &accepted.access,
        references: &earlier.resolved.references,
        derived: &earlier.resolved.derived,
        context: &earlier.resolved.context,
        relationships,
        typing: &typed.typing,
        derived_typing: &typed.derived_typing,
        classification,
    };
    build_internal_model(earlier.document, &input)
}

#[cfg(test)]
mod tests;

//! The public functions that load a model: `validate`, `load` and `with_state` (architecture
//! section 3.5, ARC-5.1, ARC-5.2).

use menura_contracts::ContractKind;
use menura_contracts::types::compiled_plan::{
    CompileErrorContract, CompileErrorDocument, Diagnostic, DiagnosticLocation, InputDocument,
};
use menura_e0_load::{LoadRejection, load_model};
use menura_model::internal_model::Warning;

use crate::catalog_state;
use crate::handle::Handle;
use crate::load_options::LoadOptions;

/// Validates the model document `model` against the catalog schema document `catalog_schema`
/// and returns the validation report (PLN-5.3): the compile error document with `contract`
/// `menura.validation_report`, whose `errors` are empty when the model loads.
///
/// Both documents are the bytes as received; their problems are diagnostics of the report.
///
/// # Example
///
/// ```
/// use menura_core::{LoadOptions, validate};
///
/// let report = validate(b"version: \"0.1.1\"", b"{}", &LoadOptions::default());
/// assert_eq!(report.errors[0].code, "ECAT-02");
/// ```
#[must_use]
pub fn validate(
    model: &[u8],
    catalog_schema: &[u8],
    options: &LoadOptions,
) -> CompileErrorDocument {
    let (errors, warnings) = match load_model(model, catalog_schema, &options.stage_options()) {
        Ok((model, _)) => (Vec::new(), diagnostics(&model.parts().warnings)),
        Err(LoadRejection { errors, warnings }) => (errors, warnings),
    };
    document(CompileErrorContract::ValidationReport, errors, warnings)
}

/// Loads the model document `model` against the catalog schema document `catalog_schema`,
/// with the catalog state document `catalog_state` when given (ARC-5.1).
///
/// The handle carries the load warnings. A catalog state that is not admitted is ignored with
/// its warning (`WCAT-01` to `WCAT-04`, ARC-5.2): the handle then has no state.
///
/// # Errors
///
/// The compile error document (PLN-5), with every error and warning E0 found, when the model
/// does not load.
pub fn load(
    model: &[u8],
    catalog_schema: &[u8],
    catalog_state: Option<&[u8]>,
    options: &LoadOptions,
) -> Result<Handle, CompileErrorDocument> {
    let (model, schema) = load_model(model, catalog_schema, &options.stage_options()).map_err(
        |LoadRejection { errors, warnings }| {
            document(CompileErrorContract::CompileError, errors, warnings)
        },
    )?;
    let warnings = diagnostics(&model.parts().warnings);
    let handle = Handle::new(model, schema, options.clone(), warnings);
    Ok(match catalog_state {
        Some(state) => with_state(&handle, state),
        None => handle,
    })
}

/// A new handle with the same loaded model and the catalog state document `catalog_state`,
/// admitted against the handle's catalog schema and model (CAT-9.1, CAT-8.8); the model is not
/// reloaded (ARC-5.1). A state that is not admitted is ignored with its warning (ARC-5.2), and
/// the new handle has no state.
#[must_use]
pub fn with_state(handle: &Handle, catalog_state: &[u8]) -> Handle {
    let state = catalog_state::admit(catalog_state, handle.catalog_schema(), handle.model());
    handle.with_catalog_state(state)
}

/// A compile error document or validation report (PLN-5).
fn document(
    contract: CompileErrorContract,
    errors: Vec<Diagnostic>,
    warnings: Vec<Diagnostic>,
) -> CompileErrorDocument {
    CompileErrorDocument {
        contract,
        contract_version: ContractKind::CompileError.latest_version(),
        errors,
        warnings,
    }
}

/// The load warnings of the internal model as diagnostics of the model document.
fn diagnostics(warnings: &[Warning]) -> Vec<Diagnostic> {
    warnings
        .iter()
        .map(|warning| Diagnostic {
            code: warning.code.as_str().to_owned(),
            message: warning.message.clone(),
            location: Some(DiagnosticLocation {
                document: InputDocument::Model,
                pointer: warning.location.pointer.clone(),
                line: u64::try_from(warning.location.line).ok(),
                column: u64::try_from(warning.location.column).ok(),
            }),
        })
        .collect()
}

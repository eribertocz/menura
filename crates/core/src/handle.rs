//! The handle `load` returns and `with_state` derives (ARC-5.1).

use std::sync::Arc;

use menura_contracts::types::catalog::{CatalogSchemaDocument, CatalogStateDocument};
use menura_contracts::types::compiled_plan::Diagnostic;
use menura_model::explanation::ExplanationEntry;
use menura_model::internal_model::InternalModel;

use crate::catalog_state::StateAdmission;
use crate::load_options::LoadOptions;

/// A loaded model: immutable (ARC-5.1). A new model means `load`; a new catalog state means
/// `with_state`, which shares the loaded model instead of reloading it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handle {
    loaded: Arc<Loaded>,
    catalog_state: Option<CatalogStateDocument>,
    state_warning: Option<Diagnostic>,
}

/// What `load` produced once and every handle derived from it shares.
#[derive(Debug, PartialEq, Eq)]
struct Loaded {
    model: InternalModel,
    catalog_schema: CatalogSchemaDocument,
    options: LoadOptions,
    warnings: Vec<Diagnostic>,
    explanation: Vec<ExplanationEntry>,
}

impl Handle {
    /// A handle without catalog state.
    pub(crate) fn new(
        model: InternalModel,
        catalog_schema: CatalogSchemaDocument,
        options: LoadOptions,
        warnings: Vec<Diagnostic>,
        explanation: Vec<ExplanationEntry>,
    ) -> Self {
        Self {
            loaded: Arc::new(Loaded {
                model,
                catalog_schema,
                options,
                warnings,
                explanation,
            }),
            catalog_state: None,
            state_warning: None,
        }
    }

    /// The same loaded model with the admitted catalog state, or without state when it was
    /// ignored (ARC-5.2), and with the state's warning.
    pub(crate) fn with_catalog_state(&self, admission: StateAdmission) -> Self {
        Self {
            loaded: Arc::clone(&self.loaded),
            catalog_state: admission.state,
            state_warning: admission.warning,
        }
    }

    /// The internal model (E0H).
    #[must_use]
    pub fn model(&self) -> &InternalModel {
        &self.loaded.model
    }

    /// The admitted catalog schema the model was loaded against.
    #[must_use]
    pub fn catalog_schema(&self) -> &CatalogSchemaDocument {
        &self.loaded.catalog_schema
    }

    /// The admitted catalog state, or `None` when none was given or it was ignored (ARC-5.2).
    #[must_use]
    pub fn catalog_state(&self) -> Option<&CatalogStateDocument> {
        self.catalog_state.as_ref()
    }

    /// The load options the model was loaded with.
    #[must_use]
    pub fn load_options(&self) -> &LoadOptions {
        &self.loaded.options
    }

    /// The decisions E0 recorded for the explanation of every plan (DEV-4, PLN-7.3).
    pub(crate) fn explanation(&self) -> &[ExplanationEntry] {
        &self.loaded.explanation
    }

    /// The load warnings (a catalog schema's `WCAT-05` first, then those of E0-4.3), then the
    /// warning of the catalog state: `WCAT-01` to `WCAT-04` for an ignored state, or `WCAT-05`
    /// for an admitted state with the former contract name (CON-0.5).
    pub fn warnings(&self) -> impl Iterator<Item = &Diagnostic> {
        self.loaded.warnings.iter().chain(&self.state_warning)
    }
}

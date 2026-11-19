//! The handle `load` returns and `with_state` derives (ARC-5.1).

use std::sync::Arc;

use semantic_compiler_contracts::types::catalog::{CatalogSchemaDocument, CatalogStateDocument};
use semantic_compiler_contracts::types::compiled_plan::Diagnostic;
use semantic_compiler_model::internal_model::InternalModel;

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
}

impl Handle {
    /// A handle without catalog state.
    pub(crate) fn new(
        model: InternalModel,
        catalog_schema: CatalogSchemaDocument,
        options: LoadOptions,
        warnings: Vec<Diagnostic>,
    ) -> Self {
        Self {
            loaded: Arc::new(Loaded {
                model,
                catalog_schema,
                options,
                warnings,
            }),
            catalog_state: None,
            state_warning: None,
        }
    }

    /// The same loaded model with `catalog_state`, or without state and with the warning of
    /// the state that was ignored (ARC-5.2).
    pub(crate) fn with_catalog_state(
        &self,
        catalog_state: Result<CatalogStateDocument, Diagnostic>,
    ) -> Self {
        let (catalog_state, state_warning) = match catalog_state {
            Ok(state) => (Some(state), None),
            Err(warning) => (None, Some(warning)),
        };
        Self {
            loaded: Arc::clone(&self.loaded),
            catalog_state,
            state_warning,
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

    /// The load warnings in the order of E0-4.3, then the warning of an ignored catalog state
    /// (`WCAT-01` to `WCAT-04`).
    pub fn warnings(&self) -> impl Iterator<Item = &Diagnostic> {
        self.loaded.warnings.iter().chain(&self.state_warning)
    }
}

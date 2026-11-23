//! Loads a generated model and compiles generated queries through the public functions of
//! `core` (R4).

use menura_core::{
    CompileErrorDocument, CompileOptions, CompileResult, Handle, LoadOptions, compile, load,
};

use crate::catalog_document::catalog_schema_document;
use crate::error::BenchmarkError;
use crate::model_document::model_document;
use crate::model_shape::ModelShape;
use crate::queries::BenchmarkQuery;
#[cfg(doc)]
use crate::queries::security_context_document;

/// The generated documents of one model, ready to load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelInputs {
    /// The name of the model.
    pub name: &'static str,
    /// The model document.
    pub model: Vec<u8>,
    /// The catalog schema document.
    pub catalog_schema: Vec<u8>,
}

impl ModelInputs {
    /// The documents of the model of `shape`.
    #[must_use]
    pub fn of(shape: &ModelShape) -> Self {
        Self {
            name: shape.name,
            model: model_document(shape),
            catalog_schema: catalog_schema_document(shape),
        }
    }

    /// Loads the model with the default load options.
    ///
    /// # Errors
    ///
    /// [`BenchmarkError::ModelRejected`] when `load` returns a compile error.
    pub fn load(&self) -> Result<Handle, BenchmarkError> {
        load(
            &self.model,
            &self.catalog_schema,
            None,
            &LoadOptions::default(),
        )
        .map_err(|error| BenchmarkError::ModelRejected {
            model: self.name,
            codes: codes(&error),
        })
    }
}

fn codes(error: &CompileErrorDocument) -> Vec<String> {
    error
        .errors
        .iter()
        .map(|diagnostic| diagnostic.code.clone())
        .collect()
}

/// Compiles `query` once with the security context document `context` (see
/// [`security_context_document`]) and the default options.
///
/// # Errors
///
/// [`BenchmarkError::QueryRejected`] when `compile` does not return a plan.
pub fn compile_query(
    handle: &Handle,
    query: &BenchmarkQuery,
    context: &[u8],
) -> Result<(), BenchmarkError> {
    match compile(handle, &query.document, context, &CompileOptions::default()).result {
        CompileResult::Plan(_) => Ok(()),
        CompileResult::Error(error) => Err(BenchmarkError::QueryRejected {
            query: query.name.clone(),
            codes: codes(&error),
        }),
    }
}

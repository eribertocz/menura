//! Pure, deterministic orchestration of the compilation stages.
//!
//! The public functions are those of architecture section 3.5 that exist in this version:
//! [`validate`], [`load`] and [`with_state`]. Their inputs are the contract documents as bytes;
//! their outputs are contract documents and the immutable [`Handle`] (ARC-5.1).
//!
//! # Example
//!
//! ```
//! use semantic_compiler_core::{CompileErrorContract, LoadOptions, load};
//!
//! let error = load(b"version: \"0.1.1\"", b"{}", None, &LoadOptions::default()).unwrap_err();
//! assert_eq!(error.contract, CompileErrorContract::CompileError);
//! ```

#![forbid(unsafe_code)]

mod catalog_state;
mod handle;
mod json;
mod load;
mod load_options;

pub use handle::Handle;
pub use json::{OutputError, document_json};
pub use load::{load, validate, with_state};
pub use load_options::{LoadOptions, LoadOptionsError};
pub use semantic_compiler_contracts::types::audit_record::OssieFormat;
pub use semantic_compiler_contracts::types::catalog::{
    CatalogSchemaDocument, CatalogStateDocument,
};
pub use semantic_compiler_contracts::types::compiled_plan::{
    CompileErrorContract, CompileErrorDocument, Diagnostic, DiagnosticLocation, InputDocument,
};
pub use semantic_compiler_model::internal_model::InternalModel;

//! Pure, deterministic orchestration of the compilation stages.
//!
//! The public functions are those of architecture section 3.5 that exist in this version:
//! [`validate`], [`load`], [`with_state`], [`compile`], [`describe`] and
//! [`integrity_checks`]. Their inputs are the contract documents as bytes; their outputs are
//! contract documents and the immutable [`Handle`] (ARC-5.1).
//!
//! # Example
//!
//! ```
//! use menura_core::{CompileErrorContract, LoadOptions, load};
//!
//! let error = load(b"version: \"0.1.1\"", b"{}", None, &LoadOptions::default()).unwrap_err();
//! assert_eq!(error.contract, CompileErrorContract::CompileError);
//! ```

#![forbid(unsafe_code)]

mod cache_key;
mod catalog_state;
mod compile;
mod compile_options;
mod describe;
mod explanation;
mod handle;
mod integrity_checks;
mod json;
mod lineage;
mod load;
mod load_options;
mod plan_document;
mod producer;
mod stop;

pub use compile::{CompileResult, compile};
pub use compile_options::CompileOptions;
pub use describe::{DescribeResult, describe};
pub use handle::Handle;
pub use integrity_checks::integrity_checks;
pub use json::{OutputError, document_json, plan_json};
pub use load::{load, validate, with_state};
pub use load_options::{LoadOptions, LoadOptionsError};
pub use menura_contracts::types::audit_record::OssieFormat;
pub use menura_contracts::types::catalog::{CatalogSchemaDocument, CatalogStateDocument};
pub use menura_contracts::types::common::Dialect;
pub use menura_contracts::types::compiled_plan::{
    CompileErrorContract, CompileErrorDocument, CompiledPlanDocument, Diagnostic,
    DiagnosticLocation, InputDocument,
};
pub use menura_contracts::types::descriptions::{
    AiContext, AmbiguousPath, DescribedDataset, DescribedField, DescribedMetric,
    IntegrityChecksDocument, MetricKind, ModelDescriptionDocument,
};
pub use menura_model::internal_model::InternalModel;
